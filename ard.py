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
    time.sleep(0.1)


    count = 0

    roll = 0
    pitch = 0
    yaw = 0

    while ser.in_waiting:
        count += 1
        c = ser.read(1)[0]
        print(ser.read(1))
        print(c)
        if count == 6:
            roll = c
        elif count == 7:
            roll <<= 8
            roll += c
            roll = ((roll & 0xFF00) >> 8) | ((roll & 0x00FF) << 8)  # Reverse the order of bytes
        elif count == 8:
            pitch += c
        elif count == 9:
            pitch <<= 8
            pitch += c
            pitch = ((pitch & 0xFF00) >> 8) | ((pitch & 0x00FF) << 8)  # Reverse the order of bytes
        elif count == 10:
            yaw += c
        elif count == 11:
            yaw <<= 8
            yaw += c
            yaw = ((yaw & 0xFF00) >> 8) | ((yaw & 0x00FF) << 8)  # Reverse the order of bytes

    print(f"Roll: {roll / 10.0} Pitch: {pitch / 10.0} Yaw: {yaw}")

def main():
    time.sleep(2)  # Allow some time for the serial connection to establish

    while True:
        data = bytearray([0])  # You can replace this with your data if needed
        # send_msp(MSP_ATTITUDE, data, len(data))
        read_data()

if __name__ == "__main__":
    main()
