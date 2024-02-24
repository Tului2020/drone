import serial
import time

# Initialize serial connection
ser = serial.Serial('/dev/ttyS0', 57600, timeout=1)  # Adjust serial port and baud rate as necessary

while True:
    # Example: Sending a command to the FC
    ser.write(b'YourCommandHere\n')
    
    # Example: Reading data from the FC
    line = ser.readline().decode('utf-8').rstrip()
    if line:
        print("Received:", line)
    
    time.sleep(1)
