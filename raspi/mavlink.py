from pymavlink import mavutil

# Create a connection to the vehicle
# Replace 'YOUR_CONNECTION_STRING' with your connection string
# For example, for a serial connection: '/dev/ttyAMA0'
# For a UDP connection: 'udp:127.0.0.1:14551'
mav_connection = mavutil.mavlink_connection('/dev/ttyS0')

# Wait for the heartbeat message to find the system ID and component ID of the vehicle
mav_connection.wait_heartbeat()
print("Heartbeat from system (system ID: %u component ID: %u)" % (mav_connection.target_system, mav_connection.target_component))

# # Example: Sending a command to change the flight mode
# # The set_mode_send function parameters may vary based on your vehicle and requirements
# # For example, to set a mode on a generic MAVLink system, use mavutil.mavlink.MAV_MODE_GUIDED
# # You may need to refer to your vehicle's documentation for specific mode values
# mav_connection.mav.set_mode_send(
#     mav_connection.target_system,
#     mavutil.mavlink.MAV_MODE_FLAG_CUSTOM_MODE_ENABLED,
#     YOUR_FLIGHT_MODE)  # Replace YOUR_FLIGHT_MODE with the desired mode

# # Check for acknowledgment
# ack_msg = mav_connection.recv_match(type='COMMAND_ACK', blocking=True)
# if ack_msg is not None:
#     print("Received acknowledgment: %s" % ack_msg)

# Close the connection
mav_connection.close()
