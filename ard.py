import serial
import time

MSP_ATTITUDE = 108

ser = serial.Serial('/dev/ttyS0', 115200)  # Replace with the correct serial port
print(ser.name)
def send_msp(message_id, data, size):
    checksum = 0

    
    ser.write(b'$M<')
    ser.write(bytes([size]))
    checksum ^= size

    ser.write(bytes([message_id]))
    checksum ^= message_id
    # print("<------------- data ---------->")
    # print("size:                    " + str(size))
    # print("checksum:                " + str(checksum))
    # print("message_id:              " + str(message_id))
    # print("checksum:                " + str(checksum))
    # print("<------------- data ---------->")

    ser.write(bytes([checksum]))

def read_data():
    while ser.in_waiting:
        incoming_bytes = ser.read(4)
        print(incoming_bytes)
        decoded_byte = incoming_bytes.decode()
        print(decoded_byte)


def main():
    time.sleep(2)  # Allow some time for the serial connection to establish

    while True:
        data = bytearray([0])  # You can replace this with your data if needed
        send_msp(MSP_ATTITUDE, data, len(data))
        time.sleep(0.1)
        read_data()

if __name__ == "__main__":
    main()
