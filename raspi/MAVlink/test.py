from pymavlink import mavutil
from time import sleep

# Set the UART port and baud rate
uart_port = '/dev/ttyS0'  # Example for Linux
# uart_port = 'COM3'  # Example for Windows
baud_rate = 57600

def main():
    # # Open the serial port
    mav_connection = mavutil.mavlink_connection(uart_port, baud=baud_rate, dialect="standard", mavversion="2.0")

    print("Waiting for MAVLink messages...")
    while True:
        try:
            # Wait for a valid MAVLink message
            msg = mav_connection.recv_match(type='COMMAND_ACK', blocking=True)
            if msg:
                print(f"Received message: {msg.to_dict()}")
        except Exception as e:
            print(f"Error: {e}")
            break

if __name__ == "__main__":
    main()
