import serial

ser = serial.Serial('/dev/ttyS0', 416666, timeout=2)
if not ser.is_open:
    ser.open()

iter = 0

try:
    while True:
        if ser.in_waiting > 0:
            received_data = ser.read(1)
            print()
            print(f'iteration: {iter}')
            print('received_data', received_data)
            print('received_data', int.from_bytes(received_data))
            if (received_data == 0xC8 or received_data == 0xEE):
                print('FOUND')

            iter += 1


except KeyboardInterrupt:
    pass