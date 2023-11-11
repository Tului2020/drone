import connection

ser = connection.ser

try:
    while True:
        if ser.in_waiting > 0:
            received_data = ser.read(4).decode('utf-8')  # Read and decode data
            print("Received:", received_data)

except KeyboardInterrupt:
    pass