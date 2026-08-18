# Player API Capability Matrix

**Status:** accepted capability boundary  
**Stability:** normative


| Capability | Kernel | Trusted controller | Player endpoint |
|---|---:|---:|---:|
| Full `EngineState` | yes | checkpoint only | no |
| Authoritative events and state delta | yes | internal/replay | no |
| Root seed and RNG counters | yes | replay/configuration | no |
| Checkpoint / restore | internal | yes | no |
| Fork | internal | yes | no |
| Authoritative replay export | internal | yes | no |
| Current observation | projection source | orchestration | yes, bound perspective |
| Information state | projection source | orchestration | yes, bound perspective |
| Visible decision | projection source | orchestration | yes, acting perspective only |
| Observed events | projection source | orchestration | yes, redacted |
| Submit response | apply | schedule | yes, own visible decision only |

A player handle is an owning shared handle, not a mutable borrow of the entire controller. Multiple player handles may coexist. Player errors are closed and identifier-free.
