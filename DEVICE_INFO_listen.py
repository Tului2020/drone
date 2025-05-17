import time
from connection import ser, PacketsTypes, crsf_validate_frame


def handleCrsfPacket(ptype, data):
    if ptype == PacketsTypes.DEVICE_INFO:
        packet = ' '.join(map(hex, data))
        print(f"Device Info: {packet}")


input = bytearray()

try:
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
            # print(f"expected_len {expected_len}")
            # print('expected_len', expected_len)
            if expected_len > 64 or expected_len < 4:
                input = []
            elif len(input) >= expected_len:
                # print(f"input {input}")
                single = input[:expected_len]  # copy out this whole packet
                input = input[expected_len:]  # and remove it from the buffer

                if not crsf_validate_frame(single):  # single[-1] != crc:
                    packet = ' '.join(map(hex, single))
                    print(f"crc error: {packet}")
                else:
                    handleCrsfPacket(single[2], single)
except KeyboardInterrupt:
    pass
