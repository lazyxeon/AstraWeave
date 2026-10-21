# Evidence: Artlist AI Toolkit generation history — AstraWeave logo + splash video

**Captured**: 2026-07-05, 21:14–21:15 local (visible in the OS clock of each screenshot).
**Supplied by**: project director (screenshots of the director's own Artlist account session), archived into the repo by the AD.1.A audit session on 2026-07-05.
**Original files**: `Screenshot 2026-07-05 2114{47,};2115{01,22}.png` from the director's `OneDrive\Pictures\Screenshots`; copied byte-identical, renamed for stability.

## What the screenshots show

All three frames are the same Artlist **AI Toolkit** generation-history session:

- URL bar: `toolkit.artlist.io/019cdd24-846d-731f-a99c-f3c5ec7503b1?mode=image&mediaTypes=generatedImage,generatedVideo` — the AI Toolkit generation workspace, not the stock catalog.
- Prompt field shows the director's own prompt: **"logo forming in space"**.
- Results grid shows multiple AstraWeave "AW" logo generations; items carry the **"Video"** badge (generated video outputs alongside generated images).
- `artlist_ai_toolkit_history_grid_1.png` (21:14:47) — grid view, three video generations visible.
- `artlist_ai_toolkit_history_single.png` (21:15:01) — single-column view of one generation.
- `artlist_ai_toolkit_history_grid_2.png` (21:15:22) — grid view, alternate scroll position (three video generations, third variant fully visible).

## What this evidences (ratified director facts, AD.1.A Task 1)

1. `assets/Astraweave_logo.jpg` and `assets/8-second_Cinematic_logo_opening.mp4` (editor splash video, loaded at `tools/aw_editor/src/splash.rs:22`) were generated via **Artlist's AI Toolkit from the director's own prompts**; the logo generation additionally used a director-owned ChatGPT-generated concept image as input.
2. Under Artlist's terms these are **"AI Output"** — rights assigned to the subscriber, unrestricted commercial use surviving subscription expiry — **not** catalog "Assets."
3. Evidence tier: **name/visual-linked** — the screenshots show the generation session and outputs visually matching the repo files; no byte-level export log links a specific generation to a specific repo file. Disclosed per the AD.1 evidence-tier discipline.

AI-generation is disclosed in `THIRD_PARTY_LICENSES.md`; copyrightability of pure AI output is contested, which is immaterial for MIT distribution of these two files.

**Scope limit (director instruction)**: this disposition covers ONLY the two files above. Any other Artlist-sourced material found in the repo requires its own Output-vs-Asset determination and is marked PENDING-DIRECTOR.
