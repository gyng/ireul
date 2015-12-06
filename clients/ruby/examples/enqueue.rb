require 'socket'
require './ireul'

buffer = open('/home/sell/howbigisthis_II.ogg', 'rb').read()
ireul = Ireul::Core.new(TCPSocket::new('127.0.0.1', 3001))
puts ireul.enqueue(buffer)
