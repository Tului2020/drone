from connection import ser

iter = 0

try:
    while True:
        if ser.in_waiting > 0:
            print(f'iteration: {iter}')
            received_data = ser.read(1)
            print('received_data', received_data)
            print('received_data', int.from_bytes(received_data))


except KeyboardInterrupt:
    pass