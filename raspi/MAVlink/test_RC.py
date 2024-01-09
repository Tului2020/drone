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
mav_connection.wait_heartbeat()
print('Connection established')

# # Function to send RC channel values
# def set_rc_channel_pwm(channel_id, pwm=1500):
#     if channel_id < 1:
#         print("Channel does not exist.")
#         return
#     else:
#         print('Valid Channel')
#     # The channel values to be sent. Channels that are not being controlled should be set to 0.
#     # In this example, we're only controlling the first channel.
#     rc_channel_values = [0] * 8
#     rc_channel_values[channel_id - 1] = pwm  # Channel IDs start at 1, but Python lists are 0-indexed

#     # Send RC_CHANNELS_OVERRIDE message
#     mav_connection.mav.rc_channels_override_send(
#         mav_connection.target_system,     # target_system
#         mav_connection.target_component,  # target_component
#         *rc_channel_values     # RC channel values (up to 8 channels)
#     )

#     print(mav_connection.recv_match(blocking=True).to_dict())
#     print(mav_connection.recv_match(type='COMMAND_ACK', blocking=True).to_dict())

def set_rc_channel_pwm(channel_id, pwm=1500):
    """ Set RC channel pwm value
    Args:
        channel_id (TYPE): Channel ID
        pwm (int, optional): Channel pwm value 1100-1900
    """
    if channel_id < 1 or channel_id > 18:
        print("Channel does not exist.")
        return

    # Mavlink 2 supports up to 18 channels:
    # https://mavlink.io/en/messages/common.html#RC_CHANNELS_OVERRIDE
    rc_channel_values = [2000 for _ in range(9)]
    rc_channel_values[channel_id - 1] = pwm
    mav_connection.mav.rc_channels_override_send(
        mav_connection.target_system,                   # target_system
        mav_connection.target_component,                # target_component
        *rc_channel_values,                             # RC channel list, in microseconds.
    )

    print(mav_connection.recv_match(type='COMMAND_ACK', blocking=True).to_dict())



# Example: Set channel 1 to 1500 PWM
set_rc_channel_pwm(1, 1600)

