# Aravath greybox progress

- [x] Phase 1 — island, ground sync, third-person camera, dig, headless tests, window stayed up
- [ ] Phase 2 — walk/run/roll motor and stamina
- [ ] Phase 3 — locomotion graph
- [ ] Phase 4 — one fight
- [ ] Phase 5 — place, support, save

Notes:

- `c++` on this machine is Clang 18, which selects a GCC 14 toolchain whose libstdc++ headers are not installed. Cargo builds of `basis-universal-sys` (pulled in by `astraweave-render`) need `CXX=g++`.
