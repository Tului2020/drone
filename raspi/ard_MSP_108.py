import time
import connection

MSP_ATTITUDE = 108

ser = connection.ser

def send_msp(cmd, data, n_bytes):
    print("sending message...")
    checksum = 0

    ser.write(b'$M<')
    ser.write(bytes([n_bytes]))
    checksum ^= n_bytes

    ser.write(bytes([cmd]))
    checksum ^= cmd

    ser.write(bytes([checksum]))

def main():
    time.sleep(2)  # Allow some time for the serial connection to establish

    while True:
        data = bytearray([0])  # You can replace this with your data if needed
        send_msp(MSP_ATTITUDE, data, len(data))
        time.sleep(1)

if __name__ == "__main__":
    main()
