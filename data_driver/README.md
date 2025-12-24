# Data-driver

Data driver for the Dario FSM contract (JSON <-> RKYV).

## ABI surface

### Functions
| name            | input JSON | output JSON |
|-----------------|------------|-------------|
| `current_state` | `null`     | number (`u32`, DarioState discriminant) |
| `revive_count`  | `null`     | number (`u32`, revival counts) |
| `handle_event`  | number (`u32`, Event discriminant) | `null` |

### Events
| name   | payload JSON |
|--------|--------------|
| `state`| number (`u32`, DarioState discriminant) |

### Enums (discriminants)
**DarioState**: `Regular=0`, `Super=1`, `Fire=2`, `Cape=3`, `GameOver=4`  
**Event**: `Espresso=0`, `ChiliPepper=1`, `TableClothCape=2`, `TakeDamage=3`, `Revive=4`

> Note: `u32` values are safe to pass as JSON **numbers**. (For `u64` you would use JSON strings, but this contract does not use `u64`.)

## Build
```
make wasm-js
```
