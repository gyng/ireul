# ireulruby

http://guides.rubygems.org/make-your-own-gem/

## Build
`gem build ./ireul.gemspec`

## Install
`gem install ./ireul-0.0.1.gem`

## Use
```
require 'socket'
require 'ireul'

buffer = open('./howbigisthis.ogg', 'rb').read()
ireul = Ireul::Core.new(TCPSocket::new('127.0.0.1', 3001))
puts ireul.enqueue(buffer)
ireul.fast_forward(:track_boundary)
```
