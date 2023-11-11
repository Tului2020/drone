import serial

ser = serial.Serial('/dev/ttyS0', 115200, timeout=0.1)
if not ser.is_open:
    ser.open()

try:
    while True:
        if ser.in_waiting > 0:
            received_data = ser.read(ser.in_waiting).decode('utf-8')  # Read and decode data
            print("Received:", received_data)

except KeyboardInterrupt:
    pass