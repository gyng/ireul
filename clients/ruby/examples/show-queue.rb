require 'socket'
require './ireul'

buffer = open('/home/sell/howbigisthis_II.ogg', 'rb').read()
ireul = Ireul::Core.new(TCPSocket::new('127.0.0.1', 3001))
qs = ireul.queue_status()

formatter = Ireul::QueueFormatter::new()
puts formatter.format(qs)
