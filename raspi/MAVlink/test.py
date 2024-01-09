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
    # mav_connection = mavutil.mavlink_connection(ser.port)
    # # For MAVLink 2
    # mavlink2_connection = mavutil.mavlink_connection(uart_port, baud=57600, dialect="standard", mavversion="2.0")

    # For MAVLink 1
    mavlink1_connection = mavutil.mavlink_connection(uart_port, baud=57600, dialect="standard", mavversion="1.0")

    print("Waiting for MAVLink messages...")
    # while True:
    try:
        # Wait for a valid MAVLink message
        msg = mavlink1_connection.recv_match(blocking=True)
        if msg:
            print(f"Received message: {msg.to_dict()}")
    except Exception as e:
        print(f"Error: {e}")
        # break

    ser.close()

if __name__ == "__main__":
    main()
