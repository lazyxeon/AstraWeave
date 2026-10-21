#!/usr/bin/env python3
"""
Third pass: fix remaining merged SIDE QUEST headers and 
break long bold-prefixed paragraphs.
"""
import re

FILE = "docs/Veilweaver/lore_bible.md"

with open(FILE, "r", encoding="utf-8") as f:
    text = f.read()

text = text.replace('\r\n', '\n')
orig_lines = text.count('\n')

# Fix remaining SIDE QUEST headers: split at "Estimated duration:"
# Handle both ASCII quotes and Unicode curly quotes
text = re.sub(
    r'^(#### SIDE QUEST: [\u201c"][^\u201d"]+[\u201d"][^\n]*?)\s+(Estimated duration:.*)$',
    lambda m: f'{m.group(1).strip()}\n\n{m.group(2)}',
    text, flags=re.MULTILINE
)

# Break long **bold** paragraphs (Beat, Rewards, etc.)
def split_bold_para(s, target_len=350):
    """Split a long line that starts with ** marker."""
    if len(s) <= target_len + 150:
        return [s]
    
    # Find sentence boundaries
    boundaries = []
    in_quote = False
    for j in range(len(s) - 2):
        c = s[j]
        if c in '\u201c\u201d"':
            in_quote = not in_quote
        if not in_quote and c in '.?!' and s[j+1] == ' ':
            next_c = s[j+2] if j+2 < len(s) else ''
            if next_c.isupper() or next_c in '\u201c"':
                boundaries.append(j + 2)
    
    if not boundaries:
        return [s]
    
    paragraphs = []
    start = 0
    for b in boundaries:
        chunk = s[start:b].strip()
        if len(chunk) >= target_len:
            paragraphs.append(chunk)
            start = b
    remainder = s[start:].strip()
    if remainder:
        if paragraphs and len(remainder) < 80:
            paragraphs[-1] += ' ' + remainder
        else:
            paragraphs.append(remainder)
    
    return paragraphs if paragraphs else [s]


lines = text.split('\n')
result = []
for line in lines:
    s = line.strip()
    if s.startswith('**') and len(s) > 500:
        parts = split_bold_para(s)
        for j, p in enumerate(parts):
            result.append(p)
            if j < len(parts) - 1:
                result.append('')
    else:
        result.append(line)
text = '\n'.join(result)

# Remove 4+ consecutive blank lines
text = re.sub(r'\n{4,}', '\n\n\n', text)
text = text.lstrip('\n')
text = text.rstrip('\n') + '\n'

with open(FILE, 'w', encoding='utf-8') as f:
    f.write(text)

final_lines = text.count('\n')
print(f"Third pass complete.")
print(f"  Before: {orig_lines} lines → After: {final_lines} lines ({final_lines - orig_lines:+d})")

# Quick stats
merged = sum(1 for l in text.split('\n') if l.strip().startswith('#### ') and len(l.strip()) > 120)
long = sum(1 for l in text.split('\n') if len(l.strip()) > 500 and not l.strip().startswith(('#', '|', '>')))
long800 = sum(1 for l in text.split('\n') if len(l.strip()) > 800 and not l.strip().startswith(('#', '|', '>')))
print(f"  Merged h4: {merged}, Long >500: {long}, Long >800: {long800}")
