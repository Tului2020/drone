# For this to work, BetaFlight needs the following configration
# 1. Ports      -> Correct UART Port needs to be configured as "Configuration/MSP"
# 2. Receiver   -> Receiver Mode also needs to be configured for "MSP (control via MSP port)"

from connection import ser
import time

MSP_RC_ID = 200
header = [36, 77, 60]
size_message_id = [16, MSP_RC_ID]

throttle = 1900
roll = 1005
pitch = 1005
yaw = 1005
aux1 = 1005
aux2 = 1005
aux3 = 1005
aux4 = 1005

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

iter = 0

while True:
    print(f"{iter} sending data...")
    iter += 1
    ser.write(bytes(message))
    time.sleep(1)
