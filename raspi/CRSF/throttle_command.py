import serial
import struct

def send_crsf_throttle_command(serial_port, throttle_value):
    """
    Send a throttle command via CRSF protocol.

    :param serial_port: The serial port connected to the device.
    :param throttle_value: Throttle value (0-1023) as an example range.
    """
    # CRSF frame structure components
    # Adjust these values based on the specific CRSF command structure
    DEST_ADDRESS = 0xEE  # Example destination address for the flight controller
    COMMAND_ID = 0x16   # Example command ID for throttle (this is hypothetical)
    payload_length = 2  # Length of the throttle value payload (in bytes)

    # Calculate CRC of the payload for error checking (simplified example)
    def crc8_dvb_s2(crc, a):
        crc ^= a
        for _ in range(8):
            if crc & 0x80:
                crc = (crc << 1) ^ 0xD5
            else:
                crc = crc << 1
        return crc & 0xFF

    # Convert throttle value to bytes
    throttle_bytes = struct.pack('<H', throttle_value)  # Little-endian format

    # Frame assembly
    frame_length = 1 + 1 + payload_length + 1  # Dest Addr + Command ID + Payload + CRC
    frame = bytearray([frame_length, DEST_ADDRESS, COMMAND_ID]) + throttle_bytes
    print("frame: {}", frame)
    # Calculate and append CRC
    crc = 0
    for b in frame:
        crc = crc8_dvb_s2(crc, b)
    frame.append(crc)

    # Send frame over serial
    serial_port.write(frame)
    print(f"Sent throttle command: {throttle_value}")

# Example usage
if __name__ == "__main__":
    # Open serial connection (adjust '/dev/ttyUSB0' and baudrate as needed)
    with serial.Serial('/dev/ttyS0', 416666, timeout=1) as ser:
        send_crsf_throttle_command(ser, 512)  # Send a mid-range throttle command
