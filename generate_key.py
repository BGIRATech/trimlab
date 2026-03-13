#!/usr/bin/env python3
"""
TrimLab — Générateur de clés de licence
Usage : python generate_key.py

⚠️  Le SECRET ici doit être IDENTIQUE à celui dans src-tauri/.cargo/config.toml
    [env]
    LICENCE_SECRET = "..."
"""

import hmac
import hashlib
import os

# ─── SEULE CHOSE À MODIFIER ────────────────────────────────────────────────
SECRET = "02525994ab93fda781accaff35a1de3dddcc9fa77704b8a19813e70fab303427"
# ───────────────────────────────────────────────────────────────────────────

BASE32 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"

def base32_encode(data: bytes) -> str:
    output, bits, value = "", 0, 0
    for byte in data:
        value = (value << 8) | byte
        bits += 8
        while bits >= 5:
            output += BASE32[(value >> (bits - 5)) & 31]
            bits -= 5
    if bits > 0:
        output += BASE32[(value << (5 - bits)) & 31]
    return output

def generate_key() -> str:
    nonce = base32_encode(os.urandom(3))[:4].upper()
    mac   = hmac.new(SECRET.encode(), nonce.encode(), hashlib.sha256).digest()
    sig   = base32_encode(mac)[:8].upper()
    return f"TRIMLAB-{nonce}-{sig[:4]}-{sig[4:8]}"

def validate_key(key: str) -> bool:
    parts = key.strip().upper().split('-')
    if len(parts) != 4 or parts[0] != 'TRIMLAB': return False
    nonce, sig_in = parts[1], parts[2] + parts[3]
    if len(nonce) != 4 or len(sig_in) != 8: return False
    mac = hmac.new(SECRET.encode(), nonce.encode(), hashlib.sha256).digest()
    return sig_in == base32_encode(mac)[:8].upper()

if __name__ == "__main__":
    print("\n=== TrimLab — Générateur de clés ===\n")
    keys = [generate_key() for _ in range(3)]
    for k in keys:
        ok = validate_key(k)
        print(f"  {k}  {'✅' if ok else '❌'}")
    print(f"\n→ Copie n'importe laquelle dans l'app pour tester.\n")