import connection
import time

ser = connection.ser

MSP_ATTITUDE = 108
message_id = MSP_ATTITUDE

byte_header = b'$M<'
byte_size = bytes([0])
byte_message_id = bytes([message_id])
byte_checksum = bytes([message_id])

print('<--------------------------------------------------->')
print('message_id       ', message_id)
print('byte_header      ', byte_header)
print('byte_size        ', byte_size)
print('byte_message_id  ', byte_message_id)
print('byte_checksum    ', byte_checksum)
print('<--------------------------------------------------->')

while True:
    print("sending data...")
    ser.write(byte_header)
    ser.write(byte_size)
    ser.write(byte_message_id)
    ser.write(byte_checksum)
    time.sleep(0.5)