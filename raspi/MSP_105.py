import connection
import time

ser = connection.ser

MSP_RC = 105
header = [36, 77, 60]
size_message_id = [16, MSP_RC]

throttle = 1000
roll = 900
pitch = 900
yaw = 900
aux1 = 1900
aux2 = 1600
aux3 = 0
aux4 = 0

payload_bytes = [
    roll.to_bytes(2, 'little'),
    pitch.to_bytes(2, 'little'),
    throttle.to_bytes(2, 'little'),
    yaw.to_bytes(2, 'little'),
    aux1.to_bytes(2, 'little'),
    aux2.to_bytes(2, 'little'),
    aux3.to_bytes(2, 'little'),
    aux4.to_bytes(2, 'little'),
]
payload = []
checksum = 0
for i in [*size_message_id, *payload_bytes]:
    if (type(i) == int):
        checksum ^= i
    else:
        payload.append(i[0])
        checksum ^= i[0]

        payload.append(i[1])
        checksum ^= i[1]

message = [*header, *size_message_id, *payload, checksum]

while True:
    print("sending data...")
    # ser.write(bytes([16, 200, 220, 5, 120, 5, 220, 5, 176, 4, 0, 0, 0, 0, 0, 0, 0, 0]))
    ser.write(bytes(message))
    time.sleep(1)
