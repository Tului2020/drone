import connection
import time

ser = connection.ser

MSP_RC = 0x69 # 105
MSP_SET_RAW_RC = 0xC8 # 200
message_id = MSP_SET_RAW_RC

# byte_header = b'$M<\x20\x69'
byte_header = b'$M<\x20\xc8'
payload = [0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0]
byte_payload = bytes(payload)

checksum = 0
checksum ^= 0x20
checksum ^= MSP_RC
for payload_byte in byte_payload:
    checksum ^= payload_byte
byte_checksum = bytes([checksum])

print('<--------------------------------------------------->')
print('byte_header      ', byte_header)
print('message_id       ', message_id)
print('payload          ', payload)
print('byte_payload     ', byte_payload)
print('checksum         ', checksum)
print('byte_checksum    ', byte_checksum)
print('<--------------------------------------------------->')

while True:
    print("sending data...")
    ser.write(byte_header)
    ser.write(byte_payload)
    ser.write(byte_checksum)
    time.sleep(0.5)