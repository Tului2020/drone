import serial
import struct

# Constants for the MultiWii Serial Protocol (MSP)
MSP_HEADER = b'$M<'

def read_message(ser):
    # Read the header
    while True:
        if ser.read() == b'$':
            if ser.read() == b'M':
                if ser.read() == b'<':
                    break

    # Read payload size and message ID
    payload_size = ord(ser.read())
    message_id = ord(ser.read())

    # Read the payload
    payload = ser.read(payload_size)

    # Read and discard checksum
    checksum = ser.read()

    return message_id, payload

def parse_payload(message_id, payload):
    # Implement the parsing logic based on the message ID
    # This is an example, you will need to adjust it based on the actual messages you expect
    if message_id == 100:  # Example message ID
        data = struct.unpack('<I', payload)  # Example: unpack an unsigned int
        return data
    else:
        return None

def main():
    # Set up the serial connection (Adjust the port and baudrate according to your setup)
    ser = serial.Serial('/dev/ttyUSB0', 115200, timeout=1)

    try:
        while True:
            message_id, payload = read_message(ser)
            data = parse_payload(message_id, payload)
            if data is not None:
                print(f'Message ID: {message_id}, Data: {data}')
            else:
                print(f'Unknown message ID: {message_id}')
    except KeyboardInterrupt:
        print('Program terminated by user')
    finally:
        ser.close()

if __name__ == '__main__':
    main()