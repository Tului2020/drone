import serial
import time
MSP_ATTITUDE = 108

# Function to send an MSP request
def send_msp_request(ser, command):
    # Construct the MSP request
    # Here, we're assuming the payload is empty, which is often the case for requests
    data = [ord('$'), ord('M'), ord('<'), 0, command, command] 
    ser.write(bytearray(data))

# Function to listen for an MSP response
def listen_msp_response(ser):
    while True:
        if ser.in_waiting > 0:
            line = ser.readline()
            print("Received:", line)
            break

# Replace '/dev/ttyUSB0' with your serial port
ser = serial.Serial('/dev/ttyS0', 115200, timeout=1)

# Example MSP command, replace 100 with your actual command code
send_msp_request(ser, MSP_ATTITUDE)

# Listen for a response
listen_msp_response(ser)

# Close the serial connection
ser.close()