import connection
import time

ser = connection.ser

try:
    while True:
        if ser.in_waiting > 0:
            received_data = ser.read(4)  # Read and decode data
            print("Received:", received_data)
            time.sleep(0.01)


except KeyboardInterrupt:
    pass