#!/usr/bin/env python3
"""
Second-pass formatting: split merged headers, break long paragraphs,
and promote Title-Case sub-headers.
"""
import re

FILE = "docs/Veilweaver/lore_bible.md"

with open(FILE, "r", encoding="utf-8") as f:
    text = f.read()

text = text.replace('\r\n', '\n')
orig_lines = text.count('\n')

# ============================================================
# PASS 1: FIX MERGED h4 HEADERS
# ============================================================

# ACT headers: split at "Main Quest:"
text = re.sub(
    r'^(#### ACT [IVX]+ \u2014 [^"]+?)\s+(Main Quest:.*)$',
    lambda m: f'{m.group(1).strip()}\n\n{m.group(2)}',
    text, flags=re.MULTILINE
)

# SIDE QUEST headers: split at "Estimated duration:"
text = re.sub(
    r'^(#### SIDE QUEST: "[^"]+"[^\n]*?)\s+(Estimated duration:.*)$',
    lambda m: f'{m.group(1).strip()}\n\n{m.group(2)}',
    text, flags=re.MULTILINE
)

# SHAR MATI: split at "Akkadian."
text = re.sub(
    r'^(#### SHAR MATI \u2014 King of the Dead Earth)\s+(Akkadian\..*)$',
    lambda m: f'{m.group(1)}\n\n{m.group(2)}',
    text, flags=re.MULTILINE
)

# ============================================================
# PASS 2: TITLE-CASE SUB-HEADERS → ##### or **bold**
# ============================================================

# These are Title Case phrases that start a body paragraph right after
# a ### or #### header. They represent sub-topics within a section.
# Pattern: After a blank line following a header, the first words are
# a Title Case phrase followed by body text.

# Known structural sub-headers that should become ##### h5:
KNOWN_SUBHEADERS = [
    'The Trinity of Magic',
    'The Primordial Cry',
    'The Inhabited Earth',
    'Before the First War',
    'The First Cataclysmic War',
    'The First Fate-Weave',
    'The Building of Milah',
    'Seven Hundred Years',
    'Those Who Came First',
    'The Thread and The Hope',
    'Core Axioms',
    'Weave Mechanics',
    'Secondary Axioms',
    'Weave Breaking',
    'Resolution',
    'The Three Layers',
    'The Cave of the Mouth',
    'Horned children of Tebel',
    'Regional Climate System',
    'The Celestial Bodies',
    'The Night Sky',
    'The Horizon Architecture',
]

lines = text.split('\n')
result = []
i = 0

while i < len(lines):
    line = lines[i]
    s = line.strip()
    
    # Check for known sub-headers at start of body text
    matched_sub = False
    if s and not s.startswith(('#', '|', '>', '**', '---', '```', '- ', '* ')):
        for sub in KNOWN_SUBHEADERS:
            if s.startswith(sub + ' ') and len(s) > len(sub) + 20:
                body = s[len(sub):].strip()
                result.append(f'##### {sub}')
                result.append('')
                result.append(body)
                matched_sub = True
                break
    
    if not matched_sub:
        result.append(line)
    i += 1

text = '\n'.join(result)

# Also handle "Dav'al's Departure" style (with apostrophe)
lines = text.split('\n')
result = []
APOS_SUBHEADERS = [
    "Dav\u2019al\u2019s Departure",
    "Dav'al's Departure",
]
for line in lines:
    s = line.strip()
    matched = False
    if s and not s.startswith('#'):
        for sub in APOS_SUBHEADERS:
            if s.startswith(sub + ' ') and len(s) > len(sub) + 20:
                body = s[len(sub):].strip()
                result.append(f'##### {sub}')
                result.append('')
                result.append(body)
                matched = True
                break
    if not matched:
        result.append(line)

text = '\n'.join(result)

# ============================================================
# PASS 3: BREAK LONG PARAGRAPHS
# ============================================================

def split_paragraph(s, target_len=350):
    """
    Break a long paragraph into multiple paragraphs at sentence boundaries.
    Target ~target_len chars per paragraph.
    """
    if len(s) <= target_len + 100:
        return [s]
    
    # Find all sentence boundaries: ". " followed by capital or quote
    # Also handle: "? " "! " ") " followed by capital
    # Don't split inside quotes (rough heuristic)
    
    boundaries = []
    in_quote = False
    for j in range(len(s) - 2):
        c = s[j]
        if c in '\u201c\u201d"':
            in_quote = not in_quote
        
        if not in_quote and c in '.?!' and s[j+1] == ' ':
            # Check next char is uppercase (sentence start)
            next_c = s[j+2] if j+2 < len(s) else ''
            if next_c.isupper() or next_c in '\u201c\u201d"':
                # Don't split after common abbreviations
                # Look back for short words before the period
                before = s[max(0, j-5):j+1].strip()
                if before.endswith(('.e.g.', '.i.e.', 'vs.', 'Dr.', 'Mr.', 'St.')):
                    continue
                boundaries.append(j + 2)  # position of the capital letter
    
    if not boundaries:
        return [s]
    
    # Group sentences into paragraphs of ~target_len chars
    paragraphs = []
    start = 0
    
    for b in boundaries:
        chunk = s[start:b].strip()
        if len(chunk) >= target_len:
            # This chunk is long enough, split here
            paragraphs.append(chunk)
            start = b
    
    # Add remainder
    remainder = s[start:].strip()
    if remainder:
        # If the remainder is very short, merge with previous
        if paragraphs and len(remainder) < 80:
            paragraphs[-1] = paragraphs[-1] + ' ' + remainder
        else:
            paragraphs.append(remainder)
    
    return paragraphs if paragraphs else [s]


lines = text.split('\n')
result = []

# Lines we should NOT split (but DO split ** bold lines if they're very long)
SKIP_STARTS = ('#', '|', '>', '---', '```', '- ', '* ', '  ')

for line in lines:
    s = line.strip()
    
    # Only process body text lines > 400 chars
    if len(s) > 400 and not any(s.startswith(p) for p in SKIP_STARTS):
        parts = split_paragraph(s)
        if len(parts) > 1:
            for j, p in enumerate(parts):
                result.append(p)
                if j < len(parts) - 1:
                    result.append('')  # blank line between paragraphs
        else:
            result.append(line)
    else:
        result.append(line)

text = '\n'.join(result)

# ============================================================
# PASS 4: ENSURE HEADER SPACING (re-run)
# ============================================================

lines = text.split('\n')
result = []
for i, line in enumerate(lines):
    s = line.strip()
    is_header = bool(re.match(r'^#{1,6}\s', s))
    if is_header:
        if result and result[-1].strip() != '':
            result.append('')
        result.append(s)
        if (i + 1 < len(lines) and 
            lines[i + 1].strip() != '' and
            not re.match(r'^#{1,6}\s', lines[i + 1].strip())):
            result.append('')
    else:
        result.append(line)
text = '\n'.join(result)

# ============================================================
# PASS 5: CLEANUP
# ============================================================

# Remove 4+ consecutive blank lines
text = re.sub(r'\n{4,}', '\n\n\n', text)
text = text.lstrip('\n')
text = text.rstrip('\n') + '\n'

# ============================================================
# WRITE & REPORT
# ============================================================

with open(FILE, 'w', encoding='utf-8') as f:
    f.write(text)

final_lines = text.count('\n')
print(f"Second pass complete.")
print(f"  Before: {orig_lines} lines")
print(f"  After:  {final_lines} lines")
print(f"  Delta:  {final_lines - orig_lines:+d} lines")

# Count headers
for level in range(1, 7):
    prefix = '#' * level + ' '
    nxt = '#' * (level + 1) + ' '
    count = sum(1 for l in text.split('\n') 
                if l.strip().startswith(prefix) and not l.strip().startswith(nxt))
    if count:
        print(f"  h{level}: {count}")

# Count remaining long paragraphs
long = sum(1 for l in text.split('\n') 
           if len(l.strip()) > 500 and not l.strip().startswith(('#', '|', '>')))
print(f"  Remaining long paragraphs (>500): {long}")
