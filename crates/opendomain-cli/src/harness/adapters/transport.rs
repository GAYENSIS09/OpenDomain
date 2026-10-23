//! Transport de processus — la seule partie qui sait comment un binaire se
//! lance sur la machine hôte. Windows : les exécuteurs npm sont des
//! `.cmd`/`.bat`, `CreateProcess` exige l'interpréteur (`cmd /C`) ; POSIX :
//! binaire direct. Ni le planning ni la matérialisation ne connaissent
//! ces détails.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};

/// Sortie capturée d'un agent CLI.
pub struct Captured {
    pub status_ok: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Exécute un CLI, capture et dévore les flux par threads, impose un
/// délai d'échéance (kill propre, pipes refermées).
pub fn run_captured(bin: &str, args: &[String], timeout_secs: u64) -> Result<Captured> {
    #[cfg(windows)]
    let mut command: Command = {
        // Leçon du 2026-10-23 (soir) : le wrap `cmd /C` retord l'argv des
        // prompts longs à guillemets et sauts de ligne — le `-m <model>`
        // épinglé n'atteignait plus le CLI, qui retombait sur son modèle
        // par défaut (un modèle du plan Go, plafonné) : « Go usage limit
        // exceeded ». On spawne le vrai binaire natif quand il existe
        // (résolution du shim npm), jamais l'interpréteur cmd.
        match resolve_editor_exe(bin) {
            Some(path) => {
                let mut c = Command::new(path);
                c.args(args);
                c
            }
            None => {
                let mut c = Command::new("cmd");
                c.arg("/C").arg(bin);
                for a in args {
                    c.arg(a);
                }
                c
            }
        }
    };
    #[cfg(not(windows))]
    let mut command: Command = {
        let mut c = Command::new(bin);
        c.args(args);
        c
    };

    let mut child: Child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("lancement de `{bin}` impossible : {e} (voir `opendomain doctor`)"))?;

    let stdout = child.stdout.take().expect("stdout pipé");
    let stderr = child.stderr.take().expect("stderr pipé");
    let t_out = drain(stdout);
    let t_err = drain(stderr);

    let start = Instant::now();
    let deadline = Duration::from_secs(timeout_secs.max(1));
    loop {
        match child.try_wait()? {
            Some(status) => {
                let out = t_out.join().unwrap_or_default();
                let err = t_err.join().unwrap_or_default();
                return Ok(Captured {
                    status_ok: status.success(),
                    stdout: out,
                    stderr: err,
                });
            }
            None if start.elapsed() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = t_out.join();
                let _ = t_err.join();
                return Err(anyhow!(
                    "`{bin}` a dépassé {timeout_secs}s — augmenter --timeout, ou demander \
explicitement le repli déterministe : `opendomain plan <slug> --offline`"
                ));
            }
            None => thread::sleep(Duration::from_millis(200)),
        }
    }
}

/// Résout le pseudo-binaire `opencode` en un vrai exécutable natif :
/// npm l'installe comme shim `.cmd` — binaire que la std refuse de
/// spawner par chemin direct (durcissement « BatBadBut ») et dont le
/// wrap par `cmd /C` retord l'argv (leçon du 2026-10-23, voir ci-dessus).
/// Le shim lui-même cite sa cible :
/// `"%dp0%\node_modules\@opencode\cli\bin\opencode.exe" %*`.
#[cfg(windows)]
fn resolve_editor_exe(bin: &str) -> Option<std::ffi::OsString> {
    use std::path::Path;
    if Path::new(bin).is_file() {
        return Some(bin.into());
    }
    let appdata = std::env::var_os("APPDATA")?;
    let shim = Path::new(&appdata).join("npm").join(format!("{bin}.cmd"));
    let text = std::fs::read_to_string(&shim).ok()?;
    let idx = text.find("\"%dp0%")?;
    let after = &text[idx + "\"%dp0%".len()..];
    let end = after.find('"')?;
    let rel = after[..end].trim_start_matches(['\\', '/']);
    let target = Path::new(&shim).parent()?.join(rel);
    if target.is_file() {
        Some(target.into_os_string())
    } else {
        None
    }
}

fn drain<R: Read + Send + 'static>(mut reader: R) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        buf
    })
}

/// `opencode run <prompt>` — mode non-interactif d'OpenCode, avec le modèle
/// **épinglé** quand le réglage `[harness] model` (ou `OPENCODE_MODEL`) en
/// donne un : la panne du modèle par défaut ne doit pas tuer plan/sonde/
/// révision — leçon du 2026-10-23 (endpoint indisponible sur le défaut).
/// Le transport reste muet : il reçoit le choix, il ne le lit pas.
pub fn opencode_run(prompt: &str, timeout_secs: u64, model: Option<&str>) -> Result<Captured> {
    let mut args = vec!["run".to_string(), prompt.to_string()];
    if let Some(m) = model {
        args.push("-m".into());
        args.push(m.to_string());
    }
    run_captured("opencode", &args, timeout_secs)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// La leçon du 2026-10-23 (soir) en régression : le transport doit
    /// spawner le binaire natif cité par le shim npm — le vrai `opencode.exe`,
    /// jamais l'interpréteur `cmd` qui retordait l'argv et perdait le `-m`.
    #[test]
    fn la_transport_resoud_le_binaire_natif_du_shim() {
        let resolved = resolve_editor_exe("opencode")
            .expect("`opencode` doit être installé via npm sur cette machine");
        let s = resolved.to_string_lossy().to_string();
        assert!(
            s.ends_with("opencode.exe") && !s.ends_with("opencode.cmd"),
            "la cible doit être le binaire natif, pas le shim : {s}"
        );
    }
}
