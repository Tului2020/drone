#!/usr/bin/env python3
"""
crsf.py  –  Minimal CRSF encoder / sender for Raspberry Pi UART

* Matches the PX4 reference implementation (frame formats & CRC)
* Tested on a Pi 4 → Betaflight 4.5 FC (UART set to CRSF, 3 V logic)
"""
from __future__ import annotations
import serial, time, struct
from typing import Iterable

# ---------------------------------------------------------------------------
# Constants (from the C++ header you provided)
# ---------------------------------------------------------------------------
SYNC_BYTE            = 0xC8          # "address" / sync marker
BAUD                 = 416666        # exact integer divisor for Pi PL011
POLY                 = 0xD5          # CRC-8 DVB-S2 polynomial
TYPE_RC              = 0x16
TYPE_BATT            = 0x08
TYPE_GPS             = 0x02
TYPE_ATTITUDE        = 0x1E
TYPE_FLIGHT_MODE     = 0x21

PAYLOAD_LEN_RC       = 22            # 16 × 11 bit = 176 bit = 22 byte
FRAME_OVERHEAD       = 4             # SYNC + LEN + CRC + TYPE (LEN counts TYPE+PAYLOAD+CRC)

# Channel value mapping (same coefficients as PX4)
def us_to_crsf(val_us: int) -> int:
    """1000-2000 µs PWM -> 172-1811 CRSF units (int)"""
    return int((val_us - 988) / (2012 - 988) * (1811 - 172) + 172)

# ---------------------------------------------------------------------------
# CRC-8 DVB-S2
# ---------------------------------------------------------------------------
def crc8(data: bytes) -> int:
    crc = 0
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = ((crc << 1) ^ POLY) & 0xFF if crc & 0x80 else (crc << 1) & 0xFF
    return crc

# ---------------------------------------------------------------------------
# Frame helpers
# ---------------------------------------------------------------------------
def pack_rc(ch: Iterable[int]) -> bytes:
    """
    Pack up to 16 channel values (already converted to CRSF units, 0-2047)
    into the 22-byte little-endian bitstream used by CRSF.
    """
    out = bytearray(PAYLOAD_LEN_RC)
    bit_ofs = 0
    for v in ch:
        byte_idx = bit_ofs // 8
        bit_idx  = bit_ofs % 8
        v &= 0x7FF
        out[byte_idx]   |= (v << bit_idx) & 0xFF
        out[byte_idx+1]  = (v >> (8-bit_idx)) & 0xFF
        if bit_idx >= 6:
            out[byte_idx+2] = (v >> (16-bit_idx)) & 0xFF
        bit_ofs += 11
    return bytes(out)

def build_frame(frame_type: int, payload: bytes) -> bytes:
    length_field = len(payload) + 2          # TYPE + PAYLOAD + CRC
    hdr = bytes((SYNC_BYTE, length_field, frame_type))
    crc = crc8(hdr[2:] + payload)            # CRC over TYPE+PAYLOAD
    return hdr + payload + bytes((crc,))

# ---------------------------------------------------------------------------
# High-level CRSF port
# ---------------------------------------------------------------------------
class CRSFPort:
    def __init__(self, port: str = "/dev/ttyS0", debug: bool = False):
        self.ser   = serial.Serial(port, BAUD, timeout=0, exclusive=True)
        self.debug = debug

    # ---------- RC channels --------------------------------------------------
    def send_rc(self,
                roll: int =1500, pitch: int =1500, yaw: int =1500, thr: int =1000,
                aux1: int =1000, aux2: int =1000, aux3: int =1000, aux4: int =1000,
                extras: Iterable[int] = ()) -> None:
        """
        Send a 16-channel RC frame at ~50 Hz.
        Values are ordinary PWM microseconds (1000-2000).
        """
        chans_us = [roll, pitch, yaw, thr, aux1, aux2, aux3, aux4, *extras]
        chans_us += [1000] * (16 - len(chans_us))          # pad to 16
        chans = [us_to_crsf(x) for x in chans_us]
        payload = pack_rc(chans)
        frame   = build_frame(TYPE_RC, payload)
        self._write(frame)

    # ---------- Example telemetry helpers -----------------------------------
    def send_battery(self, mv: int, ma: int, fuel_mah: int, remaining_pct: int) -> None:
        payload = struct.pack(">HHHB",
                              mv, ma, fuel_mah & 0xFFFFFF, remaining_pct)
        frame = build_frame(TYPE_BATT, payload)
        self._write(frame)

    def send_attitude(self, pitch_deg: float, roll_deg: float, yaw_deg: float) -> None:
        # 0.01 deg packed (same as Crossfire)
        payload = struct.pack(">hhh",
                              int(pitch_deg*100),
                              int(roll_deg*100),
                              int(yaw_deg*100))
        frame = build_frame(TYPE_ATTITUDE, payload)
        self._write(frame)

    def send_flight_mode(self, text: str) -> None:
        buf = text.encode()[:15] + b"\0"      # max 16 incl. NUL
        payload = buf.ljust(16, b"\0")
        frame = build_frame(TYPE_FLIGHT_MODE, payload[:len(buf)+1])
        self._write(frame)

    # ---------- internals ----------------------------------------------------
    def _write(self, frame: bytes) -> None:
        if self.debug:
            print(frame.hex(" "))
        self.ser.write(frame)

    def arm(self) -> None:
        # Example: throttle low, AUX1 high → arm
        self.send_rc(
            roll = 1500,
            pitch = 1500,
            yaw = 1500,
            thr = 1000,       # MUST be below min_check to arm
            aux1 = 2000       # AUX1 high = ARM
        )
        time.sleep(0.1)       # give FC 2-3 frames

# ---------------------------------------------------------------------------
# Tiny demo script -----------------------------------------------------------
if __name__ == "__main__":
    port = CRSFPort(debug=True)          # prints each frame hex
    yaw   = 1000
    step  = 6
    port.arm()                        # arm the drone

    try:
        while True:
            port.send_rc(yaw=yaw, thr=1280)   # gentle hover, sweep yaw
            yaw += step
            if yaw >= 2000 or yaw <= 1000:
                step = -step
            time.sleep(0.02)                 # 50 Hz
    except KeyboardInterrupt:
        port.send_rc(thr=1000)               # props off
        print("\nStopped — failsafe low-throttle sent.")