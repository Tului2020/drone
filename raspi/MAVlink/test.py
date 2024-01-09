from pymavlink import mavutil
from time import sleep

# Set the UART port and baud rate
uart_port = '/dev/ttyS0'  # Example for Linux
# uart_port = 'COM3'  # Example for Windows
baud_rate = 57600

def main():
    # # Open the serial port
    mav_connection = mavutil.mavlink_connection(uart_port, baud=baud_rate)

    print("Waiting for MAVLink messages...")
    while True:
        try:
            # Wait for a valid MAVLink message
            # msg = mav_connection.recv_match(type='COMMAND_ACK', blocking=True)
            msg = mav_connection.recv_match(blocking=True)
            if msg:
                msg_dict = msg.to_dict()
                if msg_dict['mavpackettype'] == 'ATTITUDE':
                    print(f"ROLL:   {msg_dict['roll']}")
                    print(f"PITCH:  {msg_dict['pitch']}")
                    print(f"YAW:    {msg_dict['yaw']}")
                    print()
        except Exception as e:
            print(f"Error: {e}")
            break

if __name__ == "__main__":
    main()
