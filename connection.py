# For this to work, BetaFlight needs the following configration
# 1. Ports      -> Correct UART Port needs to be configured as "Serial Rx"
# 2. Receiver   -> Receiver Mode also needs to be configured for "Serial (via UART)"
# 3. Receiver   -> TELEMETRY needs to be enabled

import serial
from enum import IntEnum

ser = serial.Serial('/dev/ttyS0', 416666, timeout=2)
if not ser.is_open:
    ser.open()


class PacketsTypes(IntEnum):
    GPS = 0x02                      # 2
    VARIO = 0x07                    # 7
    BATTERY_SENSOR = 0x08           # 8
    HEARTBEAT = 0x0B                # 11
    VIDEO_TRANSMITTER = 0x0F        # 15
    LINK_STATISTICS = 0x14          # 20
    RC_CHANNELS_PACKED = 0x16       # 22
    ATTITUDE = 0x1E                 # 30
    FLIGHT_MODE = 0x21              # 33
    DEVICE_INFO = 0x29              # 41
    CONFIG_READ = 0x2C              # 44
    CONFIG_WRITE = 0x2D             # 45
    RADIO_ID = 0x3A                 # 58


def crc8_dvb_s2(crc, a) -> int:
    crc = crc ^ a
    for ii in range(8):
        if crc & 0x80:
            crc = (crc << 1) ^ 0xD5
        else:
            crc = crc << 1
    return crc & 0xFF


def crc8_data(data) -> int:
    crc = 0
    for a in data:
        crc = crc8_dvb_s2(crc, a)
    return crc


def crsf_validate_frame(frame) -> bool:
    return crc8_data(frame[2:-1]) == frame[-1]


def signed_byte(b):
    return b - 256 if b >= 128 else b
