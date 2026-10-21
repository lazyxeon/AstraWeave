"""Bulk emoji replacement script for AstraWeave editor .rs files.
Replaces all emoji with professional text-based equivalents."""

import os
import re

EDITOR_SRC = os.path.join(os.path.dirname(__file__), "tools", "aw_editor", "src")

# Order matters: longer sequences first to avoid partial matches
# Variant selectors (FE0F) are handled by making them optional in the regex
REPLACEMENTS = [
    # Compound emoji (must be first)
    ("\U0001f636\u200d\U0001f32b\ufe0f", "[Haze]"),  # 😶‍🌫️

    # Transport/playback (with optional variant selector)
    ("\u25b6\ufe0f", ">"),      # ▶️
    ("\u25b6", ">"),             # ▶
    ("\u23f8\ufe0f", "||"),     # ⏸️
    ("\u23f8", "||"),            # ⏸
    ("\u23f9\ufe0f", "[]"),     # ⏹️
    ("\u23f9", "[]"),            # ⏹
    ("\u23ed\ufe0f", ">|"),     # ⏭️
    ("\u23ed", ">|"),            # ⏭
    ("\u23ea", "<<"),            # ⏪
    ("\u23e9", ">>"),            # ⏩
    ("\u23f3", "..."),           # ⏳
    ("\u23f1\ufe0f", "[Time]"), # ⏱️
    ("\u23f1", "[Time]"),       # ⏱

    # Status indicators
    ("\u2705", "[ok]"),          # ✅
    ("\u274c", "[x]"),           # ❌
    ("\u26a0\ufe0f", "[!]"),    # ⚠️
    ("\u26a0", "[!]"),           # ⚠
    # ✓ (U+2713) - keep as is, it's a standard checkmark used in professional UIs
    # ✕ (U+2715) - keep as is, it's a standard close/multiply used in professional UIs

    # Colored circles
    ("\U0001f7e1", "[Y]"),       # 🟡
    ("\U0001f534", "[R]"),       # 🔴
    ("\U0001f7e2", "[G]"),       # 🟢
    ("\U0001f535", "[B]"),       # 🔵
    ("\U0001f537", "[D]"),       # 🔷

    # Nature/Weather (with optional variant selector)
    ("\u2600\ufe0f", "[Sun]"),   # ☀️
    ("\u2600", "[Sun]"),         # ☀
    ("\U0001f319", "[Moon]"),    # 🌙
    ("\U0001f324\ufe0f", "[Clr]"), # 🌤️
    ("\U0001f324", "[Clr]"),    # 🌤
    ("\U0001f327\ufe0f", "[Rain]"), # 🌧️
    ("\U0001f327", "[Rain]"),   # 🌧
    ("\u2601\ufe0f", "[Cld]"),  # ☁️
    ("\u2601", "[Cld]"),        # ☁
    ("\U0001f32b\ufe0f", "[Fog]"), # 🌫️
    ("\U0001f32b", "[Fog]"),    # 🌫
    ("\U0001f303", "[Ngt]"),    # 🌃
    ("\U0001f305", "[Dawn]"),   # 🌅
    ("\U0001f30a", "[Wave]"),   # 🌊
    ("\U0001f33f", "[Leaf]"),   # 🌿
    ("\U0001fab6", "[Rock]"),   # 🪨  (this is not a standard Python escape)
    ("\U0001f338", "[Flwr]"),   # 🌸
    ("\U0001f333", "[Tree]"),   # 🌳
    ("\U0001fab5", "[Log]"),    # 🪵

    # Arrows (with variant selectors)
    ("\u2b06\ufe0f", "[Up]"),   # ⬆️
    ("\u2b06", "[Up]"),         # ⬆
    ("\u2b07\ufe0f", "[Dn]"),  # ⬇️
    ("\u2b07", "[Dn]"),        # ⬇
    ("\u27a1\ufe0f", "[Rt]"),  # ➡️
    ("\u27a1", "[Rt]"),        # ➡
    ("\u2b05\ufe0f", "[Lt]"),  # ⬅️
    ("\u2b05", "[Lt]"),        # ⬅
    ("\u2795", "+"),            # ➕
    ("\u2796", "-"),            # ➖

    # Objects and tools
    ("\U0001f3a8", "[Art]"),     # 🎨
    ("\U0001f3ae", "[Gp]"),     # 🎮
    ("\U0001f3b5", "[Mus]"),    # 🎵
    ("\U0001f3ac", "[Anim]"),   # 🎬
    ("\U0001f4a1", "[Lgt]"),    # 💡
    ("\U0001f4e6", "[Pkg]"),    # 📦
    ("\U0001f4ca", "[Chart]"),  # 📊
    ("\U0001f4cb", "[List]"),   # 📋
    ("\U0001f527", "[Wrn]"),    # 🔧
    ("\U0001f50d", "[Srch]"),   # 🔍
    ("\U0001f512", "[Lock]"),   # 🔒
    ("\U0001f5a5\ufe0f", "[Mon]"), # 🖥️
    ("\U0001f5a5", "[Mon]"),    # 🖥
    ("\U0001f30d", "[Glb]"),    # 🌍
    ("\U0001f3af", "[Tgt]"),    # 🎯
    ("\U0001f4a5", "[Hit]"),    # 💥
    ("\U0001f3d7", "[Bld]"),    # 🏗
    ("\U0001f525", "[Fire]"),   # 🔥
    ("\U0001f4c1", "[Dir]"),    # 📁
    ("\U0001f4c4", "[Doc]"),    # 📄
    ("\U0001f5c2\ufe0f", "[Files]"), # 🗂️
    ("\U0001f5c2", "[Files]"),  # 🗂
    ("\U0001f5bc\ufe0f", "[Img]"), # 🖼️
    ("\U0001f5bc", "[Img]"),    # 🖼
    ("\U0001f50a", "[Snd]"),    # 🔊
    ("\U0001f3b6", "[Note]"),   # 🎶
    ("\U0001f3b2", "[Dice]"),   # 🎲
    ("\U0001f4ac", "[Chat]"),   # 💬
    ("\U0001f4d0", "[Sq]"),     # 📐
    ("\U0001f578", "[Web]"),    # 🕸
    ("\U0001f465", "[Grp]"),    # 👥
    ("\U0001f52b", "[Wpn]"),    # 🔫
    ("\U0001f507", "[Mute]"),   # 🔇
    ("\U0001f4dd", "[Edit]"),   # 📝
    ("\U0001f3e0", "[Home]"),   # 🏠
    ("\U0001f5d1\ufe0f", "[Del]"), # 🗑️
    ("\U0001f5d1", "[Del]"),    # 🗑
    ("\U0001f504", "[Sync]"),   # 🔄
    ("\U0001f6d1", "[Stop]"),   # 🛑
    ("\U0001f3ad", "[Mask]"),   # 🎭
    ("\U0001f532", "[Sq]"),     # 🔲
    ("\u2b50", "[Star]"),       # ⭐
    ("\U0001f48e", "[Gem]"),    # 💎
    ("\U0001f3f7\ufe0f", "[Tag]"), # 🏷️
    ("\U0001f3f7", "[Tag]"),    # 🏷
    ("\U0001f4cc", "[Pin]"),    # 📌
    ("\U0001f52e", "[Orb]"),    # 🔮
    ("\U0001f6e1\ufe0f", "[Shld]"), # 🛡️
    ("\U0001f6e1", "[Shld]"),   # 🛡
    ("\U0001f310", "[Net]"),    # 🌐
    ("\U0001f4bb", "[PC]"),     # 💻
    ("\U0001f9e9", "[Puz]"),    # 🧩
    ("\U0001f9ea", "[Test]"),   # 🧪
    ("\U0001f517", "[Link]"),   # 🔗
    ("\U0001f4e2", "[Ann]"),    # 📢
    ("\U0001f4be", "[Save]"),   # 💾
    ("\u2728", "[Fx]"),         # ✨
    ("\U0001f680", "[Rkt]"),    # 🚀
    ("\u26a1", "[Zap]"),        # ⚡
    ("\U0001f6e0", "[Tool]"),   # 🛠
    ("\U0001f503", "[Ref]"),    # 🔃
    ("\U0001f500", "[Shuf]"),   # 🔀
    ("\U0001f4c2", "[Open]"),   # 📂

    # Characters/Entities
    ("\U0001f9e0", "[Brain]"),   # 🧠
    ("\U0001f480", "[Skull]"),   # 💀
    ("\U0001f3c3", "[Run]"),     # 🏃
    ("\U0001f916", "[Bot]"),     # 🤖
    ("\U0001f5e1", "[Swd]"),     # 🗡
    ("\U0001f9ed", "[Comp]"),    # 🧭
    ("\U0001f9f1", "[Brk]"),     # 🧱

    # Misc
    ("\U0001f3bc", "[Score]"),   # 🎼
    ("\U0001f3a4", "[Mic]"),     # 🎤
    ("\U0001f3b9", "[Keys]"),    # 🎹
    ("\U0001fa84", "[Wand]"),    # 🪄
    ("\U0001f4a0", "[Dia]"),     # 💠
    ("\U0001f4a3", "[Bomb]"),    # 💣
    ("\U0001f4a4", "[Zzz]"),     # 💤
    ("\U0001f4a8", "[Dash]"),    # 💨
    ("\U0001f4a9", "[!]"),       # 💩
    ("\U0001f4aa", "[Str]"),     # 💪
    ("\U0001f4b0", "[Gold]"),    # 💰
]

def process_file(filepath):
    """Process a single .rs file, replacing emojis."""
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    original = content
    for emoji, replacement in REPLACEMENTS:
        content = content.replace(emoji, replacement)

    if content != original:
        with open(filepath, "w", encoding="utf-8") as f:
            f.write(content)
        return True
    return False

def main():
    changed_files = []
    total_files = 0

    for root, dirs, files in os.walk(EDITOR_SRC):
        for fname in files:
            if fname.endswith(".rs"):
                total_files += 1
                filepath = os.path.join(root, fname)
                if process_file(filepath):
                    changed_files.append(os.path.relpath(filepath, EDITOR_SRC))

    print(f"Scanned {total_files} .rs files")
    print(f"Modified {len(changed_files)} files:")
    for f in sorted(changed_files):
        print(f"  {f}")

if __name__ == "__main__":
    main()
