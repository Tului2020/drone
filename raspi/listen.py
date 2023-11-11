import connection
import time

ser = connection.ser

try:
    while True:
        if ser.in_waiting > 0:
            received_data = ser.read(4).decode('utf-8')  # Read and decode data
            print("Received:", received_data)
            time.sleep(0.1)


except KeyboardInterrupt:
    pass