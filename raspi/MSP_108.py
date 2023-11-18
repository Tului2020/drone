import connection
import time

ser = connection.ser

MSP_ATTITUDE = 0x6C # 108
message_id = MSP_ATTITUDE
payload = [0]
size = len(payload)

byte_header = b'$M>'
byte_size = bytes([0])
byte_message_id = bytes([message_id])
byte_payload = bytes(payload)

checksum = 0
checksum ^= 0x20
checksum ^= message_id
for payload_byte in byte_payload:
    checksum ^= payload_byte
byte_checksum = bytes([108])

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
    ser.write(byte_header)
    ser.write(byte_size)
    ser.write(byte_message_id)
    ser.write(byte_checksum)
    time.sleep(0.5)