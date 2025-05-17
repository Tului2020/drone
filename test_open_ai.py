#!/usr/bin/env python3
"""
Send CRSF RC-channels from a Raspberry Pi over UART.

Pin-out (3 V logic!):
  Pi TX  → flight-controller RX
  Pi GND → flight-controller GND
"""

import serial, time

# --------- CRSF constants ----------------------------------------------------
SERIAL_PORT   = "/dev/ttyS0"     # enable UART in /boot/config.txt
BAUD          = 420_000            # CRSF default
ADDR_FC       = 0xC8               # “Radio TX” → flight-controller
TYPE_RC       = 0x16               # RC channels packed
MICROS_MIN    = 1000               # us range you prefer to work with
MICROS_MAX    = 2000
CRSF_MIN      = 172                # 0 % in CRSF units
CRSF_MAX      = 1811               # 100 %

# --- Helpers -----------------------------------------------------------------
def us_to_crsf(us: int) -> int:
    """Map 1000-2000 µs to 172-1811 CRSF units (11 bits)."""
    span_us   = MICROS_MAX - MICROS_MIN
    span_crsf = CRSF_MAX   - CRSF_MIN
    return int((us - MICROS_MIN) * span_crsf / span_us + CRSF_MIN)

def pack_channels(ch):
    """Pack up to 16 channels (11 bits each) into 22-byte payload."""
    payload, bitbuf, bits = bytearray(), 0, 0
    for val in ch:
        bitbuf |= (val & 0x7FF) << bits
        bits   += 11
        while bits >= 8:
            payload.append(bitbuf & 0xFF)
            bitbuf >>= 8
            bits   -= 8
    if bits:                       # final few bits
        payload.append(bitbuf)
    return payload                 # len == 22 for 16 channels

def crc8(data: bytes) -> int:      # poly 0xD5
    crc = 0
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = ((crc << 1) ^ 0xD5) & 0xFF if crc & 0x80 else (crc << 1) & 0xFF
    return crc

# --- CRSF driver -------------------------------------------------------------
class CRSF:
    def __init__(self, port=SERIAL_PORT, baud=BAUD):
        self.uart = serial.Serial(port, baud, timeout=0)

    def send_rc(self,
                roll=1500, pitch=1500, yaw=1500, thr=1000,
                aux1=1000, aux2=1000, aux3=1000, aux4=1000):
        # build 16-channel list (rest idle)
        micros   = [roll, pitch, yaw, thr,
                    aux1, aux2, aux3, aux4] + [1000]*8
        ch_vals  = [us_to_crsf(v) for v in micros]
        payload  = pack_channels(ch_vals)             # 22 bytes
        frame    = bytearray([ADDR_FC,
                              1 + len(payload) + 1,   # LEN = TYPE + PAYLOAD + CRC
                              TYPE_RC]) + payload
        frame.append(crc8(frame))
        self.uart.write(frame)

# --- Example: hold level, slow yaw sweep -------------------------------------
if __name__ == "__main__":
    crsf = CRSF()
    try:
        yaw_cmd = 1000
        direction = 1
        while True:
            crsf.send_rc(roll=1500,
                         pitch=1500,
                         yaw=yaw_cmd,
                         thr=1250)     # gentle hover
            yaw_cmd += direction * 5
            if yaw_cmd > 2000 or yaw_cmd < 1000:
                direction *= -1
            time.sleep(0.02)           # 50 Hz
    except KeyboardInterrupt:
        crsf.send_rc(thr=1000)         # safe: motors low