# Aravath greybox progress

- [x] Phase 1 — island, ground sync, third-person camera, dig, headless tests, window stayed up
- [x] Phase 2 — walk, run, sprint, jump, double jump, crouch, roll i-frames, fly toggle
- [x] Phase 3 — locomotion graph
- [x] Phase 4 — sword, shield, lock-on, enemies
- [ ] Phase 5 — place, support, save/load, nav dirty region
- [ ] Phase 6 — open-world chunk streaming
- [ ] Phase 7 — CC0 assets and credits
- [ ] Phase 8 — Windows target check

Notes:

- `c++` on this machine is Clang 18, which selects a GCC 14 toolchain whose libstdc++ headers are not installed. Cargo builds of `basis-universal-sys` (pulled in by `astraweave-render`) need `CXX=g++`.
