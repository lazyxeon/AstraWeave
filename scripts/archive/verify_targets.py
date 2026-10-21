#!/usr/bin/env python3
"""Verify that all target strings in the fix script exist in the file."""
import sys
RSQUO = '\u2019'
EMDASH = '\u2014'
LDQUO = '\u201c'
RDQUO = '\u201d'

f = open('docs/Veilweaver/lore_bible.md', 'r', encoding='utf-8').read()

targets = {
    "Kalum awareness": "Kalum knows the shadow network exists. He has known for centuries. His position is characteristic: the network provides services that the legitimate economy cannot provide, and those services serve the public interest more often than they harm it.",
    "Karu node": f"Node 6: Karu {EMDASH} The Tide Room. Concealed beneath the lighthouse keeper{RSQUO}s family home",
    "Senna weapon": f"She fights with a recurve war bow {EMDASH} the Thread Singer{RSQUO}s ranged tool, but in her hands it becomes the Thread Keeper{RSQUO}s instrument.",
    "Senna style": "Her combat style develops the Thread Keeper branch naturally: she maintains awareness of ally positions, calls out threats the player might not see, and physically positions herself between danger and whoever is most vulnerable.",
    "Shachar char": "Character: Wealthy merchant district city. The most expensive real\nestate in Tebel.",
    "Outskirts 30-50": "A rough oval road system encircling the capital region at a distance of approximately 30\\--50 miles from Milah.",
    "Ruin Islands 1": "Structures on these islands predate everything in the Keeper records. Not dwarven. Not human. Not Keh\\'Dem. Something older.",
    "Ruin Islands 2": "structures built by beings who worked with the Eyn itself, the foundational layer beneath both creation and void",
    "Ruin Islands 3": "before the first war, before Dav\\'al, before Tav\\'al, made more than an inch",
    "Stowaway 1": "because the records that would have documented their ancestry were precisely the ones the Nachash edited most aggressively. The primary Tikva bloodline\\'s records are the most corrupted files in the state genealogical office. What survived the Nachash\\'s pruning survived by becoming invisible.",
    "Stowaway 2": "the marker had evolved past their detection capability. The predator and the prey shared a city for twenty years and the predator never knew.",
    "Gideon protect": f"Gideon{RSQUO}s three-day stand included an archer rotation",
    "Asham geo": "Asham",
    "Bitu geo": "Bitu",
}

for label, target in targets.items():
    count = f.count(target)
    status = f"FOUND ({count}x)" if count > 0 else "MISS"
    print(f"  {status}: {label}")
    if count == 0:
        # Try to find nearby text
        short = target[:40]
        idx = f.find(short)
        if idx >= 0:
            print(f"    Partial match at {idx}: {repr(f[idx:idx+100])}")
        else:
            print(f"    Even short prefix not found: {repr(short)}")
