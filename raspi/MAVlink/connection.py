from pymavlink import mavutil
import time

# Define the connection string to your vehicle
# Update the connection string with your serial port and baud rate
# Set the UART port and baud rate
uart_port = '/dev/ttyS0'  # Example for Linux
# uart_port = 'COM3'  # Example for Windows
baud_rate = 57600


# Start a connection
mav_connection = mavutil.mavlink_connection(uart_port, baud=baud_rate, mavversion="2.0")
print('Connection created')

# Wait for the heartbeat message to find the system ID and component ID
print(mav_connection.wait_heartbeat())
print('Connection established')
