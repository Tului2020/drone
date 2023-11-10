import serial
import struct

def create_msp_message(code, data):
    message_length = len(data)
    checksum = message_length ^ code
    for byte in data:
        checksum ^= byte
    return bytearray([36, 77, 60, message_length, code]) + bytearray(data) + bytearray([checksum])

def send_msp_message(serial_port, code, data):
    message = create_msp_message(code, data)
    serial_port.write(message)

# Replace these values with your actual control values
pitch = 1500  # Example value for pitch
roll = 1500   # Example value for roll
throttle = 1100  # Example value for throttle
aux1 = 1000  # Example value for auxiliary 1
# Add more auxiliary channels if needed

# Convert values to byte arrays (little-endian format)
data = struct.pack('<HHHH', roll, pitch, throttle, aux1)

try:
    port = serial.Serial('/dev/ttyS0', 115200, timeout=3)  # Replace 'COM3' with your port
    msp_code = 105  # Replace with the MSP code for setting control values

    send_msp_message(port, msp_code, data)

    # Close the serial port
    port.close()

except serial.SerialException as e:
    print(f"Error: {e}")