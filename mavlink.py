import time
from pymavlink import mavutil

print("1. Creating Connection")
# Create the connection
master = mavutil.mavlink_connection("/dev/ttyS0", baud=115200)  # Update port and baud rate as needed
print("2. Connection created and sending heartbeat")
# Wait for the first heartbeat 
# This clears the message buffer and confirms the connection
master.wait_heartbeat()

print("Heartbeat from system (system %u component %u)" % (master.target_system, master.target_component))

# Function to send a test message (e.g., a ping)
def send_test_message():
    master.mav.ping_send(
        time.time(),  # Unix time
        0,  # PING sequence
        0,  # Target system
        0   # Target component
    )

# Main loop
while True:
    try:
        # Send a test message
        send_test_message()

        # Fetch new messages
        msg = master.recv_match(blocking=True)
        if not msg:
            continue
        print("Message: %s" % msg)

        # You can add custom handling for different message types here

        # Sleep for a bit before sending the next message
        time.sleep(1)

    except KeyboardInterrupt:
        break

print("Closing connection")
master.close()
