import connection
import time

ser = connection.ser

MSP_RC = 0x69 # 105
MSP_SET_RAW_RC = 0xC8 # 200
message_id = MSP_RC
payload = [0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0, 0x07, 0xD0]
size = len(payload)

byte_header = b'$M>'
byte_size = bytes([size])
byte_message_id = bytes([message_id])
byte_payload = bytes(payload)

checksum = 0
checksum ^= size
checksum ^= message_id
for payload_byte in byte_payload:
    checksum ^= payload_byte
byte_checksum = bytes([checksum])

print('<--------------------------------------------------->')
print('size             ', size)
print('payload          ', payload)
print('message_id       ', message_id)
print('checksum         ', checksum)
print()
print('byte_header      ', byte_header)
print('byte_size        ', byte_size)
print('byte_message_id  ', byte_message_id)
print('byte_payload     ', byte_payload)
print('byte_checksum    ', byte_checksum)
print('<--------------------------------------------------->')

while True:
    print("sending data...")
    ser.write(bytes([36, 77, 60, 16, 200, 220, 5, 120, 5, 176, 4, 0, 0, 0, 0, 0, 0, 0, 0, 17]))
    time.sleep(0.5)