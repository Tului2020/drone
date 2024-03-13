from board import board

try:
    while True:
        board.getData(board.ATTITUDE)
        #print (board.attitude) #uncomment for regular printing

        # Fancy printing (might not work on windows...)
        message = "angx = {:+.2f} \t angy = {:+.2f} \t heading = {:+.2f} \t elapsed = {:+.4f} \t".format(float(board.attitude['angx']),float(board.attitude['angy']),float(board.attitude['heading']),float(board.attitude['elapsed']))
        stdout.write("\r%s" % message )
        stdout.flush()
        # End of fancy printing
except Exception,error:
    print ("Error on Main: "+str(error))