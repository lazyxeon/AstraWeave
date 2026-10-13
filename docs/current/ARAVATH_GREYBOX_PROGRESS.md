# Aravath greybox progress

- [x] Phase 1 — island, ground sync, third-person camera, dig, headless tests, window stayed up
- [x] Phase 2 — walk, run, sprint, jump, double jump, crouch, roll i-frames, fly toggle
- [x] Phase 3 — locomotion graph
- [x] Phase 4 — sword, shield, lock-on, enemies
- [x] Phase 5 — place, support, save/load, nav dirty region
- [x] Phase 6 — open-world chunk streaming
- [x] Phase 7 — CC0 assets and credits
- [x] Phase 8 — Windows target check (blocked on lib.exe in this VM; recorded in the goals doc)

Demo finish (the systems pass above is not the playable demo):

- [x] Real models — rigged player, Kenney sword, shield, and enemy figurine
- [x] State-driven clips — idle, walk, run, sprint, jump, crouch, roll, attack, block, hit, death, fly
- [x] Open-world read — materials, textures, biomes, trees, rocks, foliage, sky, 1660 draw distance
- [x] HUD — health, stamina, lock-on reticle, selected block, save/load feedback
- [x] Tight controls and simple believable enemy AI
- [x] Hardening — save/load round trip, dig under the player, respawn, fly across a chunk edge, lock clears on a killing blow
- [x] Engine showcase — time-of-day sun, cascaded shadows, volumetric fog, light shafts (`docs/current/ARAVATH_DEMO.md`)

Notes:

- `c++` on this machine is Clang 18, which selects a GCC 14 toolchain whose libstdc++ headers are not installed. Cargo builds of `basis-universal-sys` (pulled in by `astraweave-render`) need `CXX=g++`.
