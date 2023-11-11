# Define the number of RC channels as specified by RC_CHANS
RC_CHANS = 16

# Create a list to represent the RC channel values
rcData = [0] * RC_CHANS

# Example values for the RC channels (replace with actual values)
# In this example, we're setting each channel to a value between 1000 and 2000
for i in range(len(rcData)):
    rcData[i] = 1500  # Replace with your desired channel values

# Print the RC channel values
for i in range(RC_CHANS):
    print(f"Channel {i + 1}: {rcData[i]}")