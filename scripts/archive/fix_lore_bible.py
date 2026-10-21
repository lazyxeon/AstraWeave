#!/usr/bin/env python3
"""
Comprehensive fix script for the Veilweaver Lore Bible.
Addresses contradictions, formatting, duplications, and plot issues.

Key encoding notes:
- File uses BOTH Unicode curly quotes (U+2019 ') and ASCII apostrophes (')
- File uses em-dash (U+2014 —) in non-escaped sections
- Book 3 sections LIV-LXI use escaped markdown: \\' \\-\\-\\- \\*\\* etc
- Book 3 sections LXII-LXIII and Books 1-2 use clean markdown with Unicode
"""

import re
import sys

FILE = r"docs\Veilweaver\lore_bible.md"

# Unicode constants
RSQUO = '\u2019'  # ' RIGHT SINGLE QUOTATION MARK (curly apostrophe)
LSQUO = '\u2018'  # ' LEFT SINGLE QUOTATION MARK
LDQUO = '\u201c'  # " LEFT DOUBLE QUOTATION MARK
RDQUO = '\u201d'  # " RIGHT DOUBLE QUOTATION MARK
EMDASH = '\u2014'  # — EM DASH

def load():
    with open(FILE, "r", encoding="utf-8") as f:
        return f.read()

def save(content):
    with open(FILE, "w", encoding="utf-8") as f:
        f.write(content)

def safe_replace(content, old, new, label="", expected=1):
    """Replace text, with validation."""
    count = content.count(old)
    if count == 0:
        print(f"  WARNING [{label}]: text not found!")
        print(f"    Looking for: {repr(old[:80])}...")
        return content
    if expected > 0 and count != expected:
        print(f"  NOTE [{label}]: found {count} occurrences (expected {expected}), replacing all")
    content = content.replace(old, new)
    print(f"  OK [{label}]: {count} replacement(s)")
    return content

def fix_seraphina_duration(content):
    """Fix Contradiction #1: Seraphina fight → 9 days consistently."""
    print("\n-- Seraphina fight: 3 days → 9 days --")

    # Part I bio (line ~62) — uses Unicode curly quotes, no escaping
    content = safe_replace(content,
        "The battle lasted three days against twelve operatives and a Kurnugi Serpent. On the third day, knowing it was ending,",
        "The battle lasted nine days against twelve operatives and a Kurnugi Serpent. On the ninth day, knowing it was ending,",
        "Seraphina bio: battle lasted", 1)

    content = safe_replace(content,
        "were created across those three days. What the kingdom calls",
        "were created across those nine days. What the kingdom calls",
        "Seraphina bio: across those days", 1)

    # --- Vignette IV (lines ~539-550, clean markdown) ---
    content = safe_replace(content,
        "Day two of three.",
        "Day seven of nine.",
        "Vignette IV: day X of Y", 1)

    content = safe_replace(content,
        "She has been fighting for thirty hours. She will fight for another forty-two.",
        "She has been fighting for six days. She will fight for another three.",
        "Vignette IV: fighting hours", 1)

    content = safe_replace(content,
        "reshaping for thirty hours into a defensive",
        "reshaping for six days into a defensive",
        "Vignette IV: reshaping hours", 1)

    # Both "dawn of day three" instances are Seraphina (lines 543, 3683)
    content = safe_replace(content,
        "dawn of day three",
        "dawn of day nine",
        "dawn of day three→nine", 2)

    content = safe_replace(content,
        "three days of woven combat and woven message",
        "nine days of woven combat and woven message",
        "Vignette IV: woven combat days", 1)

    # "three-day stand" — 5 occurrences:
    # Lines 401, 549, 2005, 2545 are Seraphina; Line 3053 is Gideon
    # Gideon's line: "Gideon's three-day stand included an archer rotation"
    # Must do Gideon FIRST (protect it), then replace all remaining
    GIDEON_MARKER = "GIDEON_THREEDAY_PROTECTED"
    content = safe_replace(content,
        f"Gideon{RSQUO}s three-day stand included an archer rotation",
        f"Gideon{RSQUO}s {GIDEON_MARKER} included an archer rotation",
        "Protect Gideon's three-day stand", 1)
    # Now replace all remaining three-day stand → nine-day stand
    content = safe_replace(content,
        "three-day stand",
        "nine-day stand",
        "Seraphina: three-day→nine-day stand", 4)
    # Restore Gideon's
    content = safe_replace(content,
        GIDEON_MARKER,
        "three-day stand",
        "Restore Gideon's three-day stand", 1)

    # Forge site  
    content = safe_replace(content,
        "Three days of Tier 4 fate-weaving",
        "Nine days of Tier 4 fate-weaving",
        "Forge site: Tier 4 days", 1)

    # Kalum journal
    content = safe_replace(content,
        "She felt it for three days",
        "She felt it for nine days",
        "Kalum journal: felt for days", 1)

    # Quest / Mechanics sections
    content = safe_replace(content,
        "across three days, compressed",
        "across nine days, compressed",
        "Mechanics: days compressed", 1)

    content = safe_replace(content,
        "three days of stand compressed",
        "nine days of stand compressed",
        "Quest Vault: stand compressed", 1)

    content = safe_replace(content,
        "three days of desperate",
        "nine days of desperate",
        "Material: desperate days", 1)

    # Note: "Carries three days" on the same line (3399) is already fixed by the above.
    # Note: "what Seraphina died doing across three days" on line 2247 is fixed by
    # "Mechanics: days compressed" above.

    return content

def fix_shadow_node_count(content):
    """Fix Contradiction #2: Shadow economy Seven → Eight nodes."""
    print("\n-- Shadow economy: 7 → 8 nodes --")
    content = safe_replace(content,
        "Seven Locations Across Tebel",
        "Eight Locations Across Tebel",
        "Shadow economy header", 1)
    return content

def fix_shachar_geography(content):
    """Fix Contradiction #3: Clarify Shachar as northern trail junction."""
    print("\n-- Shachar geographic placement --")

    # Shachar entry is in escaped Book 3 section:
    # "Shachar \-\-- The Dawn" at line ~4449
    # Direction line uses escaped dashes
    # We add junction note to the Character line
    old = "Character: Wealthy merchant district city. The most expensive real\nestate in Tebel."
    new = ("Character: Wealthy merchant district city sitting at the northern junction "
           "where the Outskirts Trail meets the Capital Road, making it both the closest "
           "trail settlement to Milah and the primary commercial gateway between the capital "
           "and the outskirts. The most expensive real\nestate in Tebel.")
    content = safe_replace(content, old, new, "Shachar: junction note", 1)

    # Also clarify the Outskirts Trail isn't a constant radius
    old2 = ("A rough oval road system encircling the capital region at a distance "
            "of approximately 30\\--50 miles from Milah.")
    new2 = ("A rough oval road system encircling the capital region. The trail\\'s distance "
            "from Milah varies significantly: at its closest, near Shachar in the northwest, "
            "it passes within 10 miles of the capital; at its farthest, in the eastern and "
            "southern stretches, it runs 40\\--50 miles out.")
    content = safe_replace(content, old2, new2, "Outskirts Trail: variable radius", 1)

    return content

def fix_aram_duration(content):
    """Fix Contradiction #4: Aram → 32 years consistently."""
    print("\n-- Aram: three decades → thirty-two years --")

    content = safe_replace(content,
        "after three decades of static",
        "after thirty-two years of static",
        "Aram: decades of static", 1)

    # 2 occurrences, both Aram-related
    content = safe_replace(content,
        "after three decades of silence",
        "after thirty-two years of silence",
        "Aram: decades of silence", 2)

    content = safe_replace(content,
        "three decades of forced inward focus",
        "thirty-two years of forced inward focus",
        "Aram: decades of focus", 1)

    content = safe_replace(content,
        "The last one three decades ago",
        "The last one thirty-two years ago",
        "Kalum: last one decades ago", 1)

    # "for three decades so you" — specific Aram ref
    content = safe_replace(content,
        "for three decades so you",
        "for thirty-two years so you",
        "Healer: decades so you", 1)

    # "for thirty years" — 3 occurrences. Only one is Aram-related (line ~2272).
    # Others are Baru (30 years running node) and someone watching (30 years).
    # Use context to target only the Aram-related one.
    content = safe_replace(content,
        "and needing in equal measure for thirty years",
        "and needing in equal measure for thirty-two years",
        "Healer: thirty years (Aram context)", 1)

    return content

def fix_senna_weapon(content):
    """Fix Plot #7: Senna weapon → war spear and shield."""
    print("\n=== PLOT FIX #7: Senna weapon → War Spear ===")

    # Header line (uses ASCII apostrophes)
    content = safe_replace(content,
        "Combat role: Support / Ranged (Recurve bow)",
        "Combat role: Support / Defensive (War spear and shield)",
        "Senna header: combat role", 1)

    # Description paragraph (uses Unicode curly apostrophes and em-dash)
    old_weapon = (
        f"She fights with a recurve war bow {EMDASH} the Thread Singer{RSQUO}s ranged tool, "
        f"but in her hands it becomes the Thread Keeper{RSQUO}s instrument. Her arrows carry "
        f"protective intent: she fires to create space for allies, to interrupt attacks targeting "
        f"companions, to establish sight lines that allow the party to reposition."
    )
    new_weapon = (
        f"She fights with a war spear and shield {EMDASH} the Thread Keeper{RSQUO}s defensive "
        f"instruments. Her spear reaches keep enemies at bay while her shield interposes between "
        f"danger and allies. She strikes to create space for the party, to interrupt attacks "
        f"targeting companions, and to hold ground that allows the others to reposition."
    )
    content = safe_replace(content, old_weapon, new_weapon, "Senna: weapon description", 1)

    # Combat style follow-up sentence (same line, Unicode quotes)
    old_style = (
        f"Her combat style develops the Thread Keeper branch naturally: she maintains awareness "
        f"of ally positions, calls out threats the player might not see, and physically positions "
        f"herself between danger and whoever is most vulnerable."
    )
    new_style = (
        f"Her combat style develops the Thread Keeper branch naturally: she maintains awareness "
        f"of ally positions, calls out threats the player might not see, and physically interposes "
        f"her shield between danger and whoever is most vulnerable. The spear{RSQUO}s reach lets "
        f"her protect without closing distance, and the shield{RSQUO}s Thread Keeper resonance "
        f"strengthens nearby allies{RSQUO} thread-connections."
    )
    content = safe_replace(content, old_style, new_style, "Senna: combat style", 1)

    # The generic "Thread Singer + Bow" mechanics section (line ~2615) stays —
    # it describes what ANY player can do with a bow, not Senna specifically.

    return content

def fix_kalum_baru_awareness(content):
    """Fix Plot #1: Kalum knows about Baru's shadow role."""
    print("\n=== PLOT FIX #1: Baru/Kalum shadow economy awareness ===")

    # The existing text already says "Kalum knows the shadow network exists."
    # We add Baru-specific awareness and Baru's justification quote.
    old = (
        "Kalum knows the shadow network exists. He has known for centuries. "
        "His position is characteristic: the network provides services that the "
        "legitimate economy cannot provide, and those services serve the public "
        "interest more often than they harm it."
    )
    new = (
        "Kalum knows the shadow network exists. He has known for centuries "
        f"{EMDASH} and he knows Baru runs the Yaqar node. When asked how a "
        f"Keeper{RSQUO}s trusted contact can also operate a shadow market, "
        f"Baru{RSQUO}s answer is direct: {LDQUO}The truth rests not only in "
        f"the light but in the shadows as well. A keeper must seek the truth "
        f"in all domains.{RDQUO} Baru{RSQUO}s shadow operations were mostly "
        f"neutral {EMDASH} frontier logistics, discreet courier work, information "
        f"that the Nachash would suppress. He was never pro-Nachash; he served "
        f"the shadows because the frontier needed services the legitimate economy "
        f"could not provide, and Kalum recognized this. His position is "
        f"characteristic: the network provides services that the legitimate "
        f"economy cannot provide, and those services serve the public interest "
        f"more often than they harm it."
    )
    content = safe_replace(content, old, new, "Kalum-Baru awareness", 1)

    return content

def fix_ruin_islands(content):
    """Fix Plot #5: Pre-creation → pre-first-war civilizations."""
    print("\n=== PLOT FIX #5: Ruin Islands → pre-first-war ===")

    # This section is in Book 3 escaped text (uses \\' and \\-\\-\\-)
    content = safe_replace(content,
        "Structures on these islands predate everything in the Keeper records. "
        "Not dwarven. Not human. Not Keh\\'Dem. Something older.",
        "Structures on these islands predate the first war and the Keeper mandate. "
        "Not dwarven. Not human. Built by a branch of the Keh\\'Dem or a related "
        "civilization that existed in the ages before the first war, when the Eyn "
        "substrate was more accessible.",
        "Ruin Islands: pre-creation", 1)

    content = safe_replace(content,
        "structures built by beings who worked with the Eyn itself, the foundational "
        "layer beneath both creation and void",
        "structures built by a pre-first-war civilization that understood the Eyn at a "
        "depth that even the modern Keh\\'Dem have not achieved",
        "Ruin Islands: substrate builders", 1)

    content = safe_replace(content,
        "before the first war, before Dav\\'al, before Tav\\'al, made more than an inch",
        "before the first war, in an age when the Eyn was more directly accessible, "
        "made more than an inch",
        "Ruin Islands: before Dav'al", 1)

    return content

def fix_stowaway_ancestry(content):
    """Fix Plot #6: Nachash knew stowaway's ancestry, dismissed him."""
    print("\n=== PLOT FIX #6: Stowaway ancestry → dismissed ===")

    # This is in Book 3 escaped text
    content = safe_replace(content,
        "because the records that would have documented their ancestry were precisely "
        "the ones the Nachash edited most aggressively. The primary Tikva bloodline\\'s "
        "records are the most corrupted files in the state genealogical office. What "
        "survived the Nachash\\'s pruning survived by becoming invisible.",

        "because the Nachash\\'s entire surveillance apparatus was calibrated for a single "
        "biological marker: horns. The stowaway\\'s ancestry was known --- the primary "
        "Tikva bloodline\\'s records were among the most scrutinized files in the state "
        "genealogical office. The Nachash identified the child, observed the absence of "
        "horns, and dismissed him as irrelevant. A hornless descendant of the Tikva "
        "bloodline was, to seven centuries of Nachash doctrine, not a Tikva and therefore "
        "not a threat. They filed the assessment and moved on. The predator saw the prey, "
        "examined it, and concluded it was not prey at all.",
        "Stowaway: dismissed not hidden", 1)

    content = safe_replace(content,
        "the marker had evolved past their detection capability. The predator and the "
        "prey shared a city for twenty years and the predator never knew.",
        "the marker had evolved into a form they recognized but dismissed. The predator "
        "and the prey shared a city for twenty years and the predator looked directly at "
        "the prey and saw nothing worth hunting.",
        "Stowaway: evolved → dismissed", 1)

    return content

def fix_lighthouse_foreshadowing(content):
    """Fix Plot #2: Add lighthouse keeper foreshadowing."""
    print("\n=== PLOT FIX #2: Lighthouse keeper foreshadowing ===")

    # Senna's section (line ~189) already establishes her as the lighthouse keeper's
    # older sister. We add the foreshadowing thread to the Karu geographic entry
    # and to the Karu shadow economy node.

    # Karu shadow node line: "Node 6: Karu — The Tide Room."
    # Add a note connecting the lighthouse keeper to Senna
    old_karu_node = (
        f"Node 6: Karu {EMDASH} The Tide Room. Concealed beneath the lighthouse "
        f"keeper{RSQUO}s family home"
    )
    new_karu_node = (
        f"Node 6: Karu {EMDASH} The Tide Room. Concealed beneath the lighthouse "
        f"keeper{RSQUO}s family home {EMDASH} the same keeper whose older sister "
        f"travels with the player{RSQUO}s party"
    )
    content = safe_replace(content, old_karu_node, new_karu_node,
        "Karu node: lighthouse sister foreshadowing", 1)

    print("  NOTE: Senna's section (XVIII) already establishes Senna as the lighthouse keeper's older sister.")

    return content

def add_geographic_entries(content):
    """Fix Plot #3-4: Add Kur Narum near Asham, Bitu Kalama under Bitu."""
    print("\n=== PLOT FIXES #3-4: Add missing locations ===")

    # These are in the Book 3 escaped geography section
    # Asham entry starts with "Asham \-\-\- The Weight" at ~line 4629
    # The next settlement after Asham is "Yeshurun"
    # Insert Kur Narum between Asham and Yeshurun

    asham_marker = "Asham \\-\\-- The Weight of What Was Done"
    next_settlement = "Yeshurun \\-\\-- The Upright Ones"
    idx_next = content.find(next_settlement)

    if idx_next > 0:
        kur_narum = (
            "Kur Narum \\-\\-- The Golden Water\n"
            "Direction and distance: South-southwest, approximately 95\\--100\n"
            "miles from Milah. Four days\\' travel.\n"
            "Terrain: Volcanic hills between Asham and the southern coast. A\n"
            "volcanic lake with bioluminescent waters nestled in a sheltered caldera.\n"
            "Character: The water glows with residual Dav magic from the first\n"
            "war \\-\\-- pre-war accounts describe it as a place where the Word\\'s warmth "
            "never fully dissipated from the volcanic substrate. Thread Sense registers "
            "the lake as the oldest continuous expression of Dav magic outside the Breath "
            "of Tebel. Sacred to the frontier communities, who call it simply \\'the golden "
            "water.\\' The Keh\\'Dem consider it a substrate bleed-through point \\-\\-- a location "
            "where the Eyn\\'s foundational warmth surfaces naturally.\n"
        )
        content = content[:idx_next] + kur_narum + content[idx_next:]
        print("  OK [Kur Narum]: Added between Asham and Yeshurun")
    elif asham_marker in content:
        # Fallback: find after Asham block
        print("  WARNING: Could not find Yeshurun marker; skipping Kur Narum")
    else:
        print("  WARNING: Could not find Asham geographic entry")

    # Bitu Kalama — insert after Bitu entry, before Shamatu
    # Bitu entry starts "Bitu \-\-\- The Home" at ~line 4615
    # Next settlement is "Shamatu \-\-\- Where Heaven"
    bitu_marker = "Bitu \\-\\-- The Home"
    shamatu_marker = "Shamatu \\-\\-- Where Heaven and Earth Meet"
    idx_shamatu = content.find(shamatu_marker)

    if idx_shamatu > 0:
        bitu_kalama = (
            "Bitu Kalama \\-\\-- The House of All That Remains\n"
            "Direction and distance: Beneath Bitu\\'s oldest quarter. Accessible\n"
            "only through concealed dwarven-engineered passages.\n"
            "Terrain: Deep underground vault complex.\n"
            "Character: A deep-chamber vault maintained by the dwarven community\n"
            "since before the first war. The vault served as the civilization\\'s emergency "
            "refuge during the worst years of the war and was never fully decommissioned. "
            "An Emberstoat colony has inhabited the vault\\'s warmest chamber for as long "
            "as any dwarf remembers \\-\\-- the oldest continuously occupied Emberstoat den "
            "in Tebel. The vault\\'s thread-signature is the oldest living connection in the "
            "underground network. Only three living dwarves know its exact location. Stocked "
            "and maintained for seven hundred years.\n"
        )
        content = content[:idx_shamatu] + bitu_kalama + content[idx_shamatu:]
        print("  OK [Bitu Kalama]: Added between Bitu and Shamatu")
    elif bitu_marker in content:
        print("  WARNING: Could not find Shamatu marker; skipping Bitu Kalama")
    else:
        print("  WARNING: Could not find Bitu geographic entry")

    return content

def fix_formatting_escapes(content):
    """Fix Formatting #1: De-escape Book 3 sections LIV-LXI."""
    print("\n=== FORMATTING FIX #1: De-escape Book 3 markdown ===")

    # Book 3 sections LIV through LXI use escaped markdown.
    # Sections LXII-LXIII already use clean markdown.
    # Find the boundary.

    # Start marker: first escaped section header (LIV at line ~4291)
    # Uses "LIV. THE TIKVA" to avoid matching the clean TOC entry "Section LIV —"
    markers = [
        "LIV. THE TIKVA",
        "LIV.",
    ]
    escaped_start = None
    for m in markers:
        idx = content.find(m)
        if idx >= 0:
            escaped_start = idx
            break

    # End marker: LXII starts clean markdown with bold prefix "**LXII."
    clean_markers = ["**LXII. THE LIVING TONGUE", "**LXII.", "LXII. THE LIVING TONGUE"]
    clean_start = None
    for m in clean_markers:
        idx = content.find(m)
        if idx >= 0:
            clean_start = idx
            break

    if escaped_start is not None and clean_start is not None and escaped_start < clean_start:
        section = content[escaped_start:clean_start]
        original_len = len(section)

        # Count escaped patterns before de-escaping
        esc_dash3 = section.count("\\-\\-\\-")
        esc_dash5 = section.count("\\-\\--")   # 5-char \-\-- (em-dash variant)
        esc_dash2 = section.count("\\-\\-")    # includes \-\-- partial matches
        esc_endash = section.count("\\--") - section.count("\\-\\-")  # standalone \-- only
        esc_apos = section.count("\\'")
        esc_bold = section.count("\\*\\*")
        esc_gt = section.count("\\>")
        esc_dbl_bs = section.count("\\\\")

        # De-escape: order matters (longer patterns first)
        section = section.replace("\\-\\-\\-", "---")  # 6-char em-dashes
        section = section.replace("\\-\\-", "--")       # 4-char en-dashes (also turns \-\-- → ---)
        section = section.replace("\\--", "--")          # 3-char en-dash escapes (e.g. 30\--50)
        section = section.replace("\\'", "'")
        section = section.replace("\\*\\*", "**")
        section = section.replace("\\>", ">")
        # Handle \\ prefixes/suffixes on section headers (\\LIX., WEATHER\\, etc.)
        section = section.replace("\\\\", "")

        new_len = len(section)
        removed = original_len - new_len
        print(f"  De-escaped: {esc_dash3} \\-\\-\\-, {esc_dash2 - esc_dash3} \\-\\-/\\-\\--, "
              f"{esc_apos} \\', {esc_bold} \\*\\*, {esc_gt} \\>, {esc_dbl_bs} \\\\")
        print(f"  Section: {original_len:,} → {new_len:,} chars ({removed:,} removed)")

        content = content[:escaped_start] + section + content[clean_start:]
    else:
        print(f"  WARNING: Could not find escaped section boundaries")
        print(f"    escaped_start={escaped_start}, clean_start={clean_start}")

    return content

def main():
    print("=" * 60)
    print("Veilweaver Lore Bible — Comprehensive Fix Script")
    print("=" * 60)

    print("\nLoading lore bible...")
    content = load()
    original_len = len(content)
    print(f"  File size: {original_len:,} characters, {content.count(chr(10)):,} lines")

    # ── Contradiction Fixes ──
    print("\n" + "=" * 40)
    print("PHASE 1: CONTRADICTION FIXES")
    print("=" * 40)
    content = fix_seraphina_duration(content)
    content = fix_shadow_node_count(content)
    content = fix_shachar_geography(content)
    content = fix_aram_duration(content)

    # ── Plot / Narrative Fixes ──
    print("\n" + "=" * 40)
    print("PHASE 2: PLOT / NARRATIVE FIXES")
    print("=" * 40)
    content = fix_senna_weapon(content)
    content = fix_kalum_baru_awareness(content)
    content = fix_lighthouse_foreshadowing(content)
    content = fix_ruin_islands(content)
    content = fix_stowaway_ancestry(content)
    content = add_geographic_entries(content)

    # ── Formatting Fixes ──
    print("\n" + "=" * 40)
    print("PHASE 3: FORMATTING FIXES")
    print("=" * 40)
    content = fix_formatting_escapes(content)

    # ── Save ──
    delta = len(content) - original_len
    print(f"\n{'=' * 40}")
    print(f"Final size: {len(content):,} chars, delta: {delta:+,}")
    print(f"New line count: {content.count(chr(10)):,}")

    # Check for any WARNINGs
    if "--dry-run" in sys.argv:
        print("\n[DRY RUN] Not saving. Review output above for any WARNINGs.")
    else:
        print("Saving...")
        save(content)
        print("Done!")

if __name__ == "__main__":
    main()
