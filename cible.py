import os
from pathlib import Path

# 🔧 ADAPTE ICI SI BESOIN
PROJECT_ROOT = Path("D:/trimlab/src")
OUTPUT_FILE = "code_dump.txt"

# Dossiers à ignorer
IGNORE_DIRS = {"node_modules", ".git", "dist", "build", "__pycache__"}

# Extensions autorisées
ALLOWED_EXT = {
    ".ts", ".js", ".svelte", ".css",
    ".json", ".html", ".d.ts"
}

def should_ignore(path: Path):
    return any(part in IGNORE_DIRS for part in path.parts)

def dump_codebase(root: Path):
    report = []

    report.append("AUTO TRIM — FULL CODEBASE DUMP")
    report.append("=" * 100)
    report.append("")

    for path in sorted(root.rglob("*")):
        if path.is_file() and not should_ignore(path):
            if path.suffix.lower() in ALLOWED_EXT:

                relative = path.relative_to(root)
                size_kb = round(path.stat().st_size / 1024, 2)

                report.append("")
                report.append("=" * 100)
                report.append(f"📄 FILE: {relative}")
                report.append(f"📏 SIZE: {size_kb} KB")
                report.append("=" * 100)
                report.append("")

                try:
                    with open(path, "r", encoding="utf-8") as f:
                        content = f.read()
                        report.append(content)
                except Exception as e:
                    report.append(f"⚠️ ERROR READING FILE: {e}")

                report.append("\n")

    return "\n".join(report)

def main():
    print("🔍 Scanning full codebase...")
    dump = dump_codebase(PROJECT_ROOT)

    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        f.write(dump)

    print(f"✅ Full dump generated: {OUTPUT_FILE}")

if __name__ == "__main__":
    main()