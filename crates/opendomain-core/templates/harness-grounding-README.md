# `grounding/` — knowledge collected at design time

"Design-time grounding": everything the harness knows about the domain,
collected **before** the agents are generated (plausible APIs, reference
examples, docs, repository conventions). One note per topic, dated and
sourced. A grounding failure at run time is typed `grounding_failure` →
replay this step, not the middle agent.
