
#!/usr/bin/env python3
import serial
import time
import argparse

from connection import PacketsTypes

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

def handleCrsfPacket(ptype, data):
    if ptype == PacketsTypes.DEVICE_INFO:
        packet = ' '.join(map(hex, data))
        print(f"Device Info: {packet}")


parser = argparse.ArgumentParser()
parser.add_argument('-P', '--port', default='/dev/ttyS0', required=False)
parser.add_argument('-b', '--baud', default=416666, required=False)
args = parser.parse_args()

with serial.Serial(args.port, args.baud, timeout=2) as ser:
    input = bytearray()
    while True:
        if ser.in_waiting > 0:
            input.extend(ser.read(ser.in_waiting))
        else:
            time.sleep(0.020)
        if len(input) > 2:
            # print('\n')
            # print('input', len(input))
            # This simple parser works with malformed CRSF streams
            # it does not check the first byte for SYNC_BYTE, but
            # instead just looks for anything where the packet length
            # is 4-64 bytes, and the CRC validates
            expected_len = input[1] + 2
            print(f"expected_len {expected_len}")
            # print('expected_len', expected_len)
            if expected_len > 64 or expected_len < 4:
                input = []
            elif len(input) >= expected_len:
                print(f"input {input}")
                single = input[:expected_len] # copy out this whole packet
                input = input[expected_len:] # and remove it from the buffer

                if not crsf_validate_frame(single): # single[-1] != crc:
                    packet = ' '.join(map(hex, single))
                    print(f"crc error: {packet}")
                else:
                    handleCrsfPacket(single[2], single)
