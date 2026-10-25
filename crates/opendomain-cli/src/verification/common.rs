//! Ce qui est partagé par les commandes du sous-système : les kinds
//! exigence-side (§9.7) et la date, calculée sans bibliothèque — un
//! enregistrement de connaissance ne doit pas dépendre d'une.

use opendomain_core::ids::IdKind;

/// The requirement-side kinds a rigour declaration answers
/// (§9.7: "made per requirement... and recorded as an artifact").
pub const REQUIREMENT_KINDS: [IdKind; 5] = [
    IdKind::Req,
    IdKind::Nfr,
    IdKind::Cnst,
    IdKind::Bas,
    IdKind::Uc,
];

// ------------------------------------------------------------------ misc ---

/// Today's date as `YYYY-MM-DD`, computed without a date library — a knowledge
/// record should not depend on one (the same stance `promotions[].date` takes).
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    civil_date(secs)
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 → civil y/m/d.
/// The March-based year trick starts years in March, so the +1 adjustment
/// applies to January and February only — the classic off-by-one here turns
/// October into next year, which is exactly the kind of wrong a test pins.
pub fn civil_date(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    if m <= 2 {
        y += 1;
    }
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_date_matches_known_days() {
        // 2026-10-22 00:00:00 UTC = 1792627200; 2026-01-01 = 1767225600;
        // 2024-02-29 (leap day) = 1709164800; 1970-01-02 = 86400.
        assert_eq!(civil_date(1_792_627_200), "2026-10-22");
        assert_eq!(civil_date(1_767_225_600), "2026-01-01");
        assert_eq!(civil_date(1_709_164_800), "2024-02-29");
        assert_eq!(civil_date(86_400), "1970-01-02");
        assert_eq!(civil_date(0), "1970-01-01");
    }
}
