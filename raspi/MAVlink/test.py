import serial
from pymavlink import mavutil

# Set the UART port and baud rate
uart_port = '/dev/ttyS0'  # Example for Linux
# uart_port = 'COM3'  # Example for Windows
baud_rate = 57600

def main():
    # Open the serial port
    ser = serial.Serial(uart_port, baud_rate, timeout=1)

    # Create a mavlink connection, specifying the serial port
    mav_connection = mavutil.mavlink_connection(ser.port)

    print("Waiting for MAVLink messages...")
    # while True:
    try:
        # Wait for a valid MAVLink message
        msg = mav_connection.recv_match(blocking=True)
        if msg:
            print(f"Received message: {msg.to_dict()}")
    except Exception as e:
        print(f"Error: {e}")
        break

    ser.close()

if __name__ == "__main__":
    main()
