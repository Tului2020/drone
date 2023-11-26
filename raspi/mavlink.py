import serial
import time
import struct

# Example CRSF message format (hypothetical)
# In a real implementation, this would need to match the CRSF protocol's specifications
# [Device Address] [Length] [Command Type] [Payload] [CRC]
crsf_msg_format = "<BBBHB"

def pack_crsf_message():
    device_address = 0xC8  # Example address
    length = 0x03  # Length of payload + 1 for command type
    command_type = 0x16  # Example command type
    payload = 0x0100  # Example payload
    crc = 0xE7  # Example CRC, would need calculation in a real scenario

    crsf_message = struct.pack(crsf_msg_format, device_address, length, command_type, payload, crc)
    return crsf_message

def unpack_crsf_message(message):
    device_address, length, command_type, payload, crc = struct.unpack(crsf_msg_format, message)
    print(f"CRSF message received: Device {device_address}, Command {command_type}, Payload {payload}")

# Setup serial connection
serial_port = "/dev/ttyS0"  # Replace with your UART port
baud_rate = 115200  # Replace with your baud rate
ser = serial.Serial(serial_port, baud_rate)

try:
    while True:
        # Send CRSF message
        crsf_message = pack_crsf_message()
        ser.write(crsf_message)

        # Check for incoming messages
        if ser.in_waiting > 0:
            data = ser.read(ser.in_waiting)
            unpack_crsf_message(data)

        time.sleep(1)

except KeyboardInterrupt:
    ser.close()
    print("Script terminated")
