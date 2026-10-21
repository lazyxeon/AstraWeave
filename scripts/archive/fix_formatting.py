#!/usr/bin/env python3
"""
Comprehensive formatting fix for lore_bible.md
Transforms the wall-of-text document into a professionally formatted markdown file.

Fixes:
1. Add proper markdown heading hierarchy (## Parts, ### Sections, #### Sub-sections)
2. Separate inline ALL-CAPS headers from body text  
3. Add blank lines before/after headers
4. Break long run-on paragraphs at natural sub-section boundaries
5. Remove redundant front-matter blocks
6. Clean up grammatical issues
"""

import re
import sys

INPUT = "docs/Veilweaver/lore_bible.md"
OUTPUT = INPUT

with open(INPUT, "r", encoding="utf-8") as f:
    text = f.read()

original_len = len(text)
original_lines = text.count('\n')

# ============================================================
# PHASE 0: Remove remaining redundant front-matter
# ============================================================

# Remove the Part I inline TOC line
text = re.sub(
    r'^(I\. The Cosmology [^\n]+)\n',
    '',
    text,
    count=1,
    flags=re.MULTILINE
)

# Remove VEILWEAVER standalone line before Part II mechanics 
text = re.sub(
    r'\nVEILWEAVER\nMECHANICS BIBLE\nFate-Weaving \u2022 Skill Trees \u2022 Weave-Building \u2022 Progression\nComplete Systems Design for AstraWeave Engine\nCompanion document to the Quest Vault \(Prologue\u2013Act IV\)\nThe thread remembers what you built\.\n',
    '\n',
    text,
    count=1
)

# Remove redundant closing matter before Part III
text = re.sub(
    r'\n\u201cThe thread remembers what you built\.\u201d\nEvery system in this document exists to serve one principle:\nthe player becomes more capable by understanding more about the world\.\nCompanion to the Quest Vault \u2022 AstraWeave Engine\n',
    '\n',
    text,
    count=1
)

# Remove redundant VEILWEAVER standalone line before Part VI
text = re.sub(
    r'\nVEILWEAVER\n(?=PART VI)',
    '\n',
    text,
    count=1
)

# ============================================================
# PHASE 1: Convert PART headers to ## headings
# ============================================================

def fix_part_headers(text):
    """Convert PART X — TITLE to ## PART X — TITLE with blank lines around."""
    def repl(m):
        header = m.group(0).strip()
        return f'\n\n## {header}\n\n'
    
    text = re.sub(
        r'^(PART [IVX]+ \u2014 [^\n]+)',
        repl,
        text,
        flags=re.MULTILINE
    )
    return text

text = fix_part_headers(text)

# ============================================================
# PHASE 2: Convert Roman numeral section headers to ### headings
# ============================================================

def fix_roman_section_headers(text):
    """
    Convert lines like:
      'I. THE COSMOLOGY The Trinity of Magic Tebel operates...'
    to:
      '### I. THE COSMOLOGY\n\nThe Trinity of Magic\n\nTebel operates...'
    And standalone section headers like:
      'XXII. SONGS OF THE THREAD'
    to:
      '### XXII. SONGS OF THE THREAD'
    """
    lines = text.split('\n')
    new_lines = []
    i = 0
    
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        
        # Match Roman numeral section headers (I. through LXIII.)
        m = re.match(
            r'^([IVXLC]+\.\s+[A-Z][A-Z\s\u2014\u2019\'\-&,]+?)(?:\s{2,}|\s+(?=[A-Z][a-z]))',
            stripped
        )
        
        if m and not stripped.startswith('#'):
            header_part = m.group(1).strip()
            rest = stripped[m.end(1):].strip()
            
            # Ensure blank line before header
            if new_lines and new_lines[-1].strip() != '':
                new_lines.append('')
            
            new_lines.append(f'### {header_part}')
            new_lines.append('')
            
            if rest:
                new_lines.append(rest)
            
            i += 1
            continue
        
        # Also catch standalone Roman numeral headers (already on own line)
        m2 = re.match(
            r'^([IVXLC]+\.\s+[A-Z][A-Z\s\u2014\u2019\'\-&,]+)$',
            stripped
        )
        if m2 and not stripped.startswith('#') and len(stripped) > 5:
            header = m2.group(1).strip()
            # Don't re-process if already has ###
            if not header.startswith('###'):
                if new_lines and new_lines[-1].strip() != '':
                    new_lines.append('')
                new_lines.append(f'### {header}')
                new_lines.append('')
                i += 1
                continue
        
        new_lines.append(line)
        i += 1
    
    return '\n'.join(new_lines)

text = fix_roman_section_headers(text)

# ============================================================
# PHASE 3: Convert known ALL-CAPS sub-section headers to #### headings
# and separate them from inline body text
# ============================================================

def fix_subsection_headers(text):
    """
    Handle patterns like:
      'DAV MAGIC — Creation The power of the spoken word...'
    Convert to:
      '#### DAV MAGIC — Creation\n\nThe power of the spoken word...'
    
    Also handle standalone ALL-CAPS lines like:
      'HIDDEN AND SECRET LOCATIONS'
    Convert to:
      '#### HIDDEN AND SECRET LOCATIONS'
    """
    lines = text.split('\n')
    new_lines = []
    
    # Patterns for known inline sub-headers with a title and body text
    # e.g., "DAV MAGIC — Creation The power of..."
    # e.g., "THE EMBERSTOAT Small. Warm russet fur..."
    # e.g., "REVAEL — The Blade Who Reads"
    
    # These are the specific region/area sub-headers
    location_subheaders = [
        'THE CAPITAL REGION', 'THE OUTSKIRTS', 'THE NORTHERN REACHES',
        'THE SOUTHERN TERRITORIES', 'THE EASTERN BORDERS', 'THE WESTERN COAST',
        'HIDDEN AND SECRET LOCATIONS', 'NATURAL LANDMARKS',
        'DAV-TOUCHED', 'TAV-TOUCHED', 'EYN-TOUCHED', 'ABZU-STIRRING',
    ]
    
    # Item categories, armor tiers, etc.
    item_categories = [
        'NAMED ARMOR SETS', 'GOOD TIER SETS', 'SUPERIOR TIER SETS',
        'EPIC TIER SETS', 'LEGENDARY TIER SETS', 'PRIMORDIAL TIER SETS',
        'THE SUBSTRATE SET', 'NEW WEAPON CATEGORIES', 'ARMORY OVERVIEW',
        'MELEE WEAPONS', 'RANGED WEAPONS', 'ARMOR', 'KEY ITEMS & ARTIFACTS',
        'CRAFTING MATERIALS', 'CONSUMABLES', 'BUILDING MATERIAL QUALITY',
        'QUALITY TIER SYSTEM', 'RINGS', 'NECKLACES', 'RELICS',
    ]
    
    # Mechanical/design headers
    design_headers = [
        'DESIGN PHILOSOPHY', 'ARCHERY & STEALTH SYSTEMS',
        'COLOPHON', 'COMPLETE STATUS',
    ]
    
    # ACT/MOVEMENT/PHASE/LAYER headers  
    act_headers_re = re.compile(
        r'^(ACT [IVX]+|MOVEMENT [IVX]+|LAYER (?:ONE|TWO|THREE|1|2|3)|EPILOGUE|PROLOGUE)'
        r'(\s+\u2014\s+[^\n]+)?$'
    )
    
    phase_headers_re = re.compile(
        r'^PHASE (ONE|TWO|THREE|FOUR|FIVE|SIX)(\s+\u2014\s+[^\n]+)?$'
    )
    
    for i, line in enumerate(lines):
        stripped = line.strip()
        
        if not stripped or stripped.startswith('#'):
            new_lines.append(line)
            continue
        
        # Skip lines that are clearly body text (starts lowercase or with quote)
        if stripped[0].islower() or stripped[0] in '""\u201c\u201d':
            new_lines.append(line)
            continue
        
        handled = False
        
        # 1. ACT/MOVEMENT/PHASE/LAYER headers (standalone)
        if act_headers_re.match(stripped) or phase_headers_re.match(stripped):
            if new_lines and new_lines[-1].strip() != '':
                new_lines.append('')
            new_lines.append(f'#### {stripped}')
            new_lines.append('')
            handled = True
        
        # 2. Inline sub-headers: ALL-CAPS NAME followed by body text
        # Pattern: "NAME — Subtitle Body text here..."
        # or: "NAME Body text here..."
        if not handled:
            # Match: CAPS_TITLE [— Subtitle] Body...
            m = re.match(
                r'^([A-Z][A-Z\s\'\u2019\u2014—\-&,\.]+?)(?:\s+\u2014\s+|\s+---\s+)'
                r'([A-Z][^\n]{0,60}?)\s+'
                r'([A-Z][a-z][^\n]+)$',
                stripped
            )
            if m:
                name = m.group(1).strip()
                subtitle = m.group(2).strip()
                body = m.group(3).strip()
                header = f'{name} \u2014 {subtitle}'
                
                if new_lines and new_lines[-1].strip() != '':
                    new_lines.append('')
                new_lines.append(f'#### {header}')
                new_lines.append('')
                new_lines.append(body)
                handled = True
        
        if not handled:
            # Match: CAPS_NAME BodyText (no subtitle)
            # e.g., "DWARVES Co-survivors of the first war..."
            # e.g., "THE EMBERSTOAT Small. Warm russet fur..."
            m = re.match(
                r'^([A-Z][A-Z\s\'\u2019&,]+?)\s+'
                r'([A-Z][a-z][^\n]{20,})$',
                stripped
            )
            if m:
                name = m.group(1).strip()
                body = m.group(2).strip()
                
                # Only treat as header if name looks like a title (>3 chars, mostly caps)
                if len(name) > 3 and name == name.upper():
                    if new_lines and new_lines[-1].strip() != '':
                        new_lines.append('')
                    new_lines.append(f'#### {name}')
                    new_lines.append('')
                    new_lines.append(body)
                    handled = True
        
        if not handled:
            # 3. Standalone ALL-CAPS headers (own line, no body text)
            if (stripped == stripped.upper() and 
                re.search(r'[A-Z]{3,}', stripped) and
                len(stripped) > 5 and len(stripped) < 200 and
                not stripped.startswith('|') and
                not stripped.startswith('DEF:') and
                not stripped.startswith('---')):
                
                # Check if it's a known category or a general title-like line
                is_known = any(stripped.startswith(cat) for cat in 
                              location_subheaders + item_categories + design_headers)
                is_titlelike = (len(stripped.split()) <= 12 and 
                               not any(c.isdigit() for c in stripped[:3]))
                
                if is_known or is_titlelike:
                    if new_lines and new_lines[-1].strip() != '':
                        new_lines.append('')
                    new_lines.append(f'#### {stripped}')
                    new_lines.append('')
                    handled = True
        
        if not handled:
            new_lines.append(line)
    
    return '\n'.join(new_lines)

text = fix_subsection_headers(text)

# ============================================================
# PHASE 4: Break apart sub-sections within long blocks
# These are Title Case headers inline with body text
# e.g., "The Three-Part Battle Dav'al speaks..."
# e.g., "Core Axioms Every mechanical system..."
# ============================================================

def fix_titlecase_inline_headers(text):
    """
    Find Title Case phrases at the start of lines or mid-sentence that
    function as sub-section headers, and break them onto their own line.
    """
    lines = text.split('\n')
    new_lines = []
    
    # Known title-case sub-headers that appear inline
    known_subheaders = [
        'The Trinity of Magic', 'The Primordial Cry', 'The Inhabited Earth',
        'Before the First War', 'The First Cataclysmic War',
        'The First Fate-Weave', 'The Building of Milah',
        "Dav\u2019al\u2019s Departure", "Dav'al's Departure", 'Seven Hundred Years',
        'Those Who Came First', 'The Three-Part Battle', 'The Final Weave',
        'The Aftermath', 'The Convergence',
        'Who Walks With the Seventh', 'Companion Design Philosophy',
        'The Three Who Were There First',
        'System Overview', 'The Trunk: Core Tiers',
        'Core Axioms', 'The Three Pillars',
        'No Grinding. No Filler. No Padding.',
        'The Stamina Economy', 'The Yahwey Mechanic',
        'Post-Completion', 'Voxel System Requirements',
        'Thread Sense as GOAP Input', 'Mutation Testing Implications',
        'Local LLM Integration',
        'The Memory Interface', 'The No-Save Covenant',
        'Hidden Discovery Layers', 'Universal Vignette Mechanics',
        'The Dark Souls Contract', 'Fate-Weaving as Progression',
        'Reassembly',
        'The First Council', 'The Crowning',
        'The Reign of the Patient King',
        "How Kalum Governs \u2014 And Why It Works",
        'The Open Record', 'The Distributed Council',
        'The Dwarven Return', "Kalum\u2019s Personal Style",
        "Kalum's Personal Style",
        'The King and the Tikva',
        'The Recipe Book',
        'The Forge',
        "The Tanner\u2019s Frame", "The Tanner's Frame",
        'The Apothecary Table',
        'The Loom',
        'The Stonemason Table',
    ]
    
    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith('#'):
            new_lines.append(line)
            continue
        
        # Try to find known sub-headers inline at the START of a line
        found = False
        for header in known_subheaders:
            if stripped.startswith(header + ' ') and len(stripped) > len(header) + 20:
                rest = stripped[len(header):].strip()
                # Check that what follows looks like body text  
                if rest and rest[0].isupper():
                    if new_lines and new_lines[-1].strip() != '':
                        new_lines.append('')
                    new_lines.append(f'##### {header}')
                    new_lines.append('')
                    new_lines.append(rest)
                    found = True
                    break
            elif stripped == header:
                # Already on its own line, just add heading markup
                if new_lines and new_lines[-1].strip() != '':
                    new_lines.append('')
                new_lines.append(f'##### {header}')
                new_lines.append('')
                found = True
                break
        
        if not found:
            new_lines.append(line)
    
    return '\n'.join(new_lines)

text = fix_titlecase_inline_headers(text)

# ============================================================
# PHASE 5: Fix the Fallen Tikva entries — separate name/bio pairs
# Pattern: "DEVORAH — The Scholar Who Remembered First Tikva | Female..."
# These should be #### headers with the bio as body text
# ============================================================

# Already handled by Phase 3 for the CAPS entries

# ============================================================
# PHASE 6: Fix specific inline patterns in LAYER headers that
# were merged with body on same line  
# ============================================================

# LAYER patterns: "LAYER 1 — The Monster Darkness. Then a sound..."
for layer_num, layer_name in [('1', 'The Monster'), ('2', 'The Campsite'), 
                               ('3', 'The Tavern (Real)')]:
    pattern = f'LAYER {layer_num} \u2014 {layer_name} '
    if pattern in text:
        idx = text.index(pattern)
        # Find the rest of the content
        header_end = idx + len(pattern)
        # Find end of header (next sentence start)
        body_start = header_end
        header = f'LAYER {layer_num} \u2014 {layer_name}'
        body = text[body_start:].split('\n')[0]
        old = f'{header} {body}'
        new = f'\n\n#### {header}\n\n{body}'
        text = text.replace(old, new, 1)

# Also fix "LAYER ONE/TWO/THREE" variants
for layer_word in ['ONE', 'TWO', 'THREE']:
    pattern_re = re.compile(
        rf'^(LAYER {layer_word}\s+\u2014\s+[^\n]{{5,60}}?)\s+([A-Z][a-z][^\n]+)$',
        re.MULTILINE
    )
    text = pattern_re.sub(r'\n\n#### \1\n\n\2', text)

# ============================================================
# PHASE 7: Break long run-on paragraphs at natural sub-topics
# Look for mid-paragraph Title Case headers that mark topic shifts
# ============================================================

def break_mid_paragraph_headers(text):
    """
    Find patterns where a sub-topic header appears mid-sentence:
    ...end of previous topic. New Topic Name Body text continues...
    
    Split these at the topic boundary.
    """
    # Common patterns to look for mid-line:
    # - "sentence end. Topic Name Sentence start" where Topic Name is 2-4 Title Case words
    
    # Specific known mid-paragraph breaks
    mid_breaks = [
        # Format: (search_text, header, body_start_word)
        ('. The Attack ', 'The Attack', 'The '),
        ('. Design Integration ', 'Design Integration', ''),
        ('. The Stamina Economy ', 'The Stamina Economy', ''),
    ]
    
    for search, header, body_prefix in mid_breaks:
        if search in text:
            parts = text.split(search, 1)
            if len(parts) == 2:
                body = parts[1] if not body_prefix else body_prefix + parts[1]
                text = parts[0] + f'.\n\n##### {header}\n\n' + body
    
    return text

text = break_mid_paragraph_headers(text)

# ============================================================
# PHASE 8: Clean up the title/opening section
# ============================================================

# Convert the static opening to proper markdown
text = re.sub(
    r'^# Table of Contents\n'
    r'THE RESOLUTE WEAVE\n'
    r'The Complete Lore Bible of Veilweaver\n',
    '# THE RESOLUTE WEAVE\n\n'
    '## The Complete Lore Bible of Veilweaver\n\n',
    text
)

# Make MASTER TABLE OF CONTENTS a proper heading
text = re.sub(
    r'^MASTER TABLE OF CONTENTS$',
    '### Master Table of Contents',
    text,
    flags=re.MULTILINE
)

# ============================================================
# PHASE 9: Clean up multi-header runs  
# (Phase 2-3 may have created ### followed by #### for same section)
# ============================================================

# Remove excessive blank lines (3+ consecutive -> max 2)
text = re.sub(r'\n{4,}', '\n\n\n', text)

# Remove blank lines between a heading and its immediate sub-heading
# e.g., ### Section\n\n\n#### Sub -> ### Section\n\n#### Sub
text = re.sub(r'(^#{2,6} [^\n]+)\n\n\n(#{2,6} )', r'\1\n\n\2', text, flags=re.MULTILINE)

# ============================================================
# PHASE 10: Handle the \# table header that markdownlint flags
# ============================================================
text = text.replace('\n\\#\n', '\n**\\#**\n')

# ============================================================
# PHASE 11: Fix specific remaining issues
# ============================================================

# Remove "In collaboration with Andrew" that appears mid-document (keep only first and last)
lines = text.split('\n')
collab_indices = [i for i, l in enumerate(lines) 
                  if 'In collaboration with Andrew' in l]
if len(collab_indices) > 2:
    # Keep first and last only
    to_remove = collab_indices[1:-1]
    for idx in reversed(to_remove):
        lines.pop(idx)
    text = '\n'.join(lines)

# ============================================================
# PHASE 12: Ensure proper spacing around all headers
# ============================================================

def ensure_header_spacing(text):
    """Ensure blank line before and after every markdown heading."""
    lines = text.split('\n')
    new_lines = []
    
    for i, line in enumerate(lines):
        stripped = line.strip()
        is_header = stripped.startswith('#') and ' ' in stripped[:7]
        
        if is_header:
            # Ensure blank line before (unless at start or previous is blank)
            if new_lines and new_lines[-1].strip() != '':
                new_lines.append('')
            new_lines.append(line)
            # Peek at next line - if not blank and not another header, add blank
            if i + 1 < len(lines) and lines[i + 1].strip() != '' and not lines[i + 1].strip().startswith('#'):
                new_lines.append('')
        else:
            new_lines.append(line)
    
    return '\n'.join(new_lines)

text = ensure_header_spacing(text)

# ============================================================
# PHASE 13: Final cleanup
# ============================================================

# Remove excessive blank lines again
text = re.sub(r'\n{4,}', '\n\n\n', text)

# Ensure file ends with single newline
text = text.rstrip('\n') + '\n'

# ============================================================
# Write output and report
# ============================================================

with open(OUTPUT, "w", encoding="utf-8") as f:
    f.write(text)

final_len = len(text)
final_lines = text.count('\n')

print(f"Done!")
print(f"  Original: {original_lines} lines, {original_len:,} chars")
print(f"  Final:    {final_lines} lines, {final_len:,} chars")
print(f"  Delta:    {final_lines - original_lines:+d} lines, {final_len - original_len:+d} chars")

# Count headers by level
for level in range(1, 7):
    prefix = '#' * level + ' '
    count = len([l for l in text.split('\n') if l.strip().startswith(prefix) and not l.strip().startswith('#' * (level+1))])
    if count:
        print(f"  h{level} headers: {count}")
