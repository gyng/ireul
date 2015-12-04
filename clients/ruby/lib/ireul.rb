require 'forwardable'
require 'stringio'

module Ireul
  TYPE_ARRAY = 0x0000
  TYPE_BLOB = 0x0002
  TYPE_STRUCT = 0x0005
  TYPE_VOID = 0x0080
  TYPE_U16 = 0x0081
  TYPE_U32 = 0x0082
  TYPE_U64 = 0x0083
  TYPE_STRING = 0x0084
  TYPE_RESULT_OK = 0x0085
  TYPE_RESULT_ERR = 0x0086

  TYPESET_NUMBER = [
    TYPE_U16,
    TYPE_U32,
    TYPE_U64,
  ]
  TYPESET_ALL = [
    TYPE_ARRAY,
    TYPE_BLOB,
    TYPE_STRUCT,
    TYPE_VOID,
    TYPE_U16,
    TYPE_U32,
    TYPE_U64,
    TYPE_STRING,
    TYPE_RESULT_OK,
    TYPE_RESULT_ERR,
  ]

  def self._expect_type(reader, expected_types)
    type_short = reader.read(2).unpack('n')[0]
    if not expected_types.include?(type_short)
      allowed = expected_types.join(', ')
      raise KeyError, "Unexpected type: got #{type_short} expected #{allowed}"
    end
    type_short
  end

  def self._pack_u32(val)
    [Ireul::TYPE_U32, val].pack('nN')
  end

  def self._pack_blob(blob)
    io = StringIO::new()
    io.write([
      Ireul::TYPE_BLOB,
      blob.size
    ].pack('nN'))
    io.write(blob)
    io.string
  end

  def self._pack_string(blob)
    io = StringIO::new()
    io.write([
      Ireul::TYPE_STRING,
      blob.size
    ].pack('nN'))
    io.write(blob)
    io.string
  end

  def self._unpack_instance(reader, allow_types=nil)
    if nil == allow_types
      allow_types = Ireul::TYPESET_ALL
    end

    type_id = Ireul::_expect_type(reader, allow_types)
    case type_id
    when TYPE_ARRAY
      length = reader.read(4).unpack('N')[0]
      (0...length).map { Ireul::_unpack_instance(reader) }.to_a
    when TYPE_BLOB
      length = reader.read(4).unpack('N')[0]
      reader.read(length)
    when TYPE_STRUCT
      length = reader.read(4).unpack('N')[0]
      out = Hash::new()
      length.times {
        key = Ireul::_unpack_instance(reader, [Ireul::TYPE_STRING]).to_sym()
        val = Ireul::_unpack_instance(reader)
        out[key] = val
      }
      out
    when TYPE_VOID
      Ireul::Unit
    when TYPE_U16
      reader.read(2).unpack('n')[0]
    when TYPE_U32
      reader.read(4).unpack('N')[0]
    when TYPE_U64
      reader.read(8).unpack('Q>')[0]
    when TYPE_STRING
      length = reader.read(4).unpack('N')[0]
      reader.read(length)
    when TYPE_RESULT_OK
      raise StandardError, "unimplemented"
    when TYPE_RESULT_ERR
      raise StandardError, "unimplemented"
    end
  end
end

module Ireul
  class Unit
    class << self
      def new()
        return Unit
      end
    end

    def self.from_frame(buffer)
      Ireul::_expect_type(buffer, [TYPE_VOID])
    end
  end

  class HashError < StandardError
    def new(hash)
      super("Error")
      @hash = hash
    end

    def self.from_frame(buffer)
      HashError::new(Ireul::_unpack_instance(buffer, [TYPE_STRUCT]))
    end
  end

  class Result
    class Bound
      def from_frame(buffer)
        result = Result::allocate()
        type_id = Ireul::_expect_type(buffer, [TYPE_RESULT_OK, TYPE_RESULT_ERR])
        case type_id
        when TYPE_RESULT_OK
          ok_fac = @ok_fac
          result.instance_eval {
            @ok = ok_fac.call(buffer)
          }
        when TYPE_RESULT_ERR
          err_fac = @err_fac
          result.instance_eval {
            @err = err_fac.call(buffer)
          }
        end
        result
      end
    end

    def unwrap()
      self.raise_error()
      @ok
    end

    def raise_error()
      # FIXME: this should only be defined for Results whose @err
      # are subclasses of StandardError
      if @err != nil
        raise @err
      end
    end

    def self.create_type(ok_type, err_type)
      bound = Bound::allocate()
      bound.instance_eval {
        @ok_fac = ok_type.method(:from_frame)
        @err_fac = err_type.method(:from_frame)
      }
      bound
    end
  end

  module Enum
    def self.included base
      base.extend ClassMethods
    end

    module ClassMethods
      def from_id(id)
        for variant in self::VARIANTS
          if variant::VARIANT_ID == id
            return variant
          end
        end
        raise KeyError, "variant #{id} not found"
      end

      def from_symbol(sym)
        self::SYMBOLS[sym]
      end
    end
  end

  module RequestType
    include Enum

    module VariantMixin
      def self.included base
        base.extend ClassMethods
      end

      module ClassMethods
        def to_int()
          self::VARIANT_ID
        end
      end
    end

    module EnqueueTrack
      include VariantMixin

      VARIANT_ID = 0x1000
    end

    module FastForward
      include VariantMixin

      VARIANT_ID = 0x1001
    end

    module QueueStatus
      include VariantMixin

      VARIANT_ID = 0x1002
    end

    VARIANTS = [
      EnqueueTrack,
      FastForward,
      QueueStatus,
    ]

    SYMBOLS = {
      :enqueue_track => EnqueueTrack,
      :fast_forward => FastForward,
      :queue_status => QueueStatus,
    }
  end

  module FastForward
    include Enum

    module VariantMixin
      def self.included base
        base.extend ClassMethods
      end

      module ClassMethods
        def to_int()
          self::VARIANT_ID
        end
      end
    end

    module TrackBoundary
      include VariantMixin

      VARIANT_ID = 0
    end

    VARIANTS = [
      TrackBoundary,
    ]

    SYMBOLS = {
      :track_boundary => TrackBoundary,
    }
  end

  class Handle
    def self.from_frame(buffer)
      value = Ireul::_unpack_instance(buffer, [Ireul::TYPE_U64])
      handle = Handle::allocate()
      handle.instance_eval {
        @value = value
      }
      handle
    end

    attr_reader :value

    def to_s()
      "Handle(#{@value.to_s(16)})"
    end

    def inspect()
      "Handle(#{@value})"
    end
  end

  module EnqueueTrackError
    include Enum

    class InvalidTrack < StandardError
      VARIANT_ID = 1

      attr_reader :message
    end

    class BadSampleRate < StandardError
      VARIANT_ID = 2

      attr_reader :message
    end

    class Full < StandardError
      VARIANT_ID = 3

      attr_reader :message
    end

    VARIANTS = [
      InvalidTrack,
      BadSampleRate,
      Full,
    ]

    def self.from_frame(buffer)
      variant_id = Ireul::_unpack_instance(buffer, Ireul::TYPESET_NUMBER)
      variant_type = EnqueueTrackError::from_id(variant_id)
      variant_type::new()
    end
  end

  module FastForwardError
    include Enum

    VARIANTS = []

    def self.from_frame(buffer)
      variant_id = Ireul::_unpack_instance(buffer, Ireul::TYPESET_NUMBER)

      variant_type = FastForwardError::from_id(variant_id)
      msg_len = buffer.read(4).unpack('N')[0]
      variant_type::new(buffer.read(msg_len))
    end

    def self.from_id(id)
      for variant in VARIANTS
        if variant::VARIANT_ID == id
          return variant
        end
      end
      raise KeyError, "variant #{id} not found"
    end
  end

  class UnknownError < StandardError
  end

  class InvalidAddress < StandardError
  end

  class ProtocolError < StandardError
  end

  class UnknownHandle < StandardError
  end

  class PlayQueue
    attr_reader :tracks # Array<Track>

    def initialize()
      @tracks = Array::new()
    end
  end

  class Track
    # an opaque (to the client) u64 allowing queue set operations
    attr_reader :handle

    # track artist
    attr_reader :artist
    # track album
    attr_reader :album
    # track title
    attr_reader :title
    # extended metadata, in a Hash
    attr_reader :extended

    # the sample rate in Hz
    attr_reader :sample_rate
    # the number of samples in the currently playing track
    attr_reader :sample_count
    # the number of samples that have been played. This is always
    # zero if the song is in queue.
    attr_reader :sample_position

    def self.from_frame(buffer)
      hash = Ireul::_unpack_instance(buffer,
        allow_types=[Ireul::TYPE_STRUCT])
      Track::from_hash(hash)
    end

    def self.from_hash(hash)
      track = Track::allocate()
      track.instance_eval {
        @handle = hash[:handle],

        @artist = hash[:artist]
        @album = hash[:album]
        @title = hash[:title]
        @extended = hash[:extended]

        @sample_rate = hash[:sample_rate]
        @sample_count = hash[:sample_count]
        @sample_position = hash[:sample_position]
      }
      track
    end

    def position()
      @sample_position.to_f / @sample_rate
    end

    def duration()
      # in seconds
      @sample_count.to_f / @sample_rate
    end
  end

  class Queue
    attr_reader :init_time
    attr_reader :tracks

    def initialize(tracks)
      @init_time = Time.now
      @tracks = tracks.map do |track|
        QueueTrack::new(self, track)
      end

      @tracks.each do |track|
        track.instance_eval do
          queue_insert_posthook()
        end
      end
    end

    def self.wrap_tracks(tracks)
      queue = Queue::allocate()
      tracks = tracks.map do |track|
        QueueTrack::new(queue, track)
      end

      queue.instance_eval do
        @init_time = Time.now
        @tracks = tracks
      end
      tracks.each do |track|
        track.instance_eval {
          queue_insert_posthook()
        }
      end
      tracks
    end
  end

  class QueueTrack
    extend Forwardable

    def_delegators :@track,
      :artist, :album, :title, :extended,
      :sample_rate, :sample_count, :sample_position,
      :position, :duration

    attr_reader :start_time

    def initialize(queue, track)
      @queue = queue
      @track = track
      @start_time = nil
    end

    private
    def queue_insert_posthook()
      @start_time = get_start_time()
    end

    def get_start_time()
      tracks = @queue.tracks
      found_at = -1

      tracks.each_with_index do |value, i|
        if value.equal?(self)
          found_at = i
          break
        end
      end

      if found_at < 0
        raise KeyError, "track not found in queue"
      end

      tracks_before = [found_at, 0].max
      previous_tracks = tracks[0...tracks_before]
      queue_pos = previous_tracks.map(&:position).inject(0, &:+)
      queue_dur = previous_tracks.map(&:duration).inject(0, &:+)

      @queue.init_time + queue_dur - queue_pos - self.position()
    end
  end

  class QueueStatus
    attr_reader :queue

    def self.from_frame(buffer)
      hash = Ireul::_unpack_instance(buffer,
        allow_types=[Ireul::TYPE_STRUCT])
      QueueStatus::from_hash(hash)
    end

    def self.from_hash(hash)
      status = QueueStatus::allocate()
      status.instance_eval {
        @queue = Queue::wrap_tracks(hash[:upcoming]
          .map {|h| Track::from_hash(h) })
      }
      status
    end

    def current()
      self.queue[0]
    end

    def upcoming()
      self.queue[1..-1]
    end
  end

  class Core
    ENQUEUE_RESPONSE_TYPE = Result::create_type(Handle, EnqueueTrackError)
    FAST_FORWARD_RESPONSE_TYPE = Result::create_type(Unit, FastForwardError)
    QUEUE_STATUS_RESPONSE_TYPE = Result::create_type(QueueStatus, HashError)

    def initialize(socket)
      @socket = socket
    end

    # Accepts an ogg file in the form of a string
    def enqueue(track) # -> Handle
      io = StringIO::new()
      io.write([Ireul::TYPE_STRUCT, 1].pack('nN'))
      io.write(Ireul::_pack_string("track"))
      io.write(Ireul::_pack_blob(track))

      send_frame(RequestType::EnqueueTrack, io.string)
      rx_frame = recv_frame()
      frame = StringIO::new(rx_frame)
      result = self.class::ENQUEUE_RESPONSE_TYPE.from_frame(frame)

      if frame.pos != frame.size
        raise StandardError, "Frame not consumed"
      end

      result.unwrap()
    end

    # Accepts a value in Ireul::FastForward::SYMBOLS and performs that kind of fast forward.
    # Currently, only fast forwarding to the next track boundary is supported.
    def fast_forward(ff_type)
      fast_forward = FastForward::from_symbol(ff_type)
      if fast_forward == nil
        allowed_values = FastForward::SYMBOLS.keys().map(&:inspect).join(", ")
        raise KeyError, "fast forward type #{ff_type.inspect} not found. Allowed values: #{allowed_values}"
      end

      io = StringIO::new()
      io.write([Ireul::TYPE_STRUCT, 1].pack('nN'))
      io.write(Ireul::_pack_string("kind"))
      io.write(Ireul::_pack_u32(fast_forward.to_int))

      send_frame(RequestType::FastForward, io.string)
      rx_frame = recv_frame()
      frame = StringIO::new(rx_frame)
      result = self.class::FAST_FORWARD_RESPONSE_TYPE.from_frame(frame)


      if frame.pos != frame.size
        raise StandardError, "Frame not consumed"
      end

      result.unwrap()
    end

    def queue_status()
      io = StringIO::new()
      io.write([Ireul::TYPE_STRUCT, 0].pack('nN'))
      send_frame(RequestType::QueueStatus, io.string)

      rx_frame = recv_frame()
      frame = StringIO::new(rx_frame)
      result = self.class::QUEUE_STATUS_RESPONSE_TYPE.from_frame(frame)

      if frame.pos != frame.size
        raise StandardError, "Frame not consumed"
      end

      result.unwrap()
    end

    private

    def send_frame(req_type, frame)
      req_header = [
        0,  # version
        req_type.to_int(),  # operation
        frame.bytesize(),  # message size
      ].pack('CNN')
      @socket.write(req_header)
      @socket.write(frame)
    end

    def recv_frame()
      header = @socket.recv(4)
      frame_length = header.unpack('N')[0]
      if frame_length > 0
        @socket.recv(frame_length)
      else
        ""
      end
    end
  end

  class QueueFormatter
    def format(queue)
      io = StringIO::new()
      io.write("=== NOW PLAYING ===\n")
      io.write("#{queue.current.start_time} :: #{queue.current.artist} - #{queue.current.title}\n")
      io.write("=== UPCOMING ===\n")
      for item in queue.upcoming
        io.write("#{item.start_time} :: #{item.artist} - #{item.title}\n")
      end
      io.string
    end
  end

  module Tests
    def self.all_tests()
      Tests.methods.find_all {|x|
        x.to_s.start_with?("test_")
      }.map {|x| Tests::method(x) }
    end

    def self.run_tests()
      out = Hash::new()
      Tests.methods.find_all {|x|
        x.to_s.start_with?("test_")
      }.each {|x|
        begin
          Tests::method(x).()
          out[x] = :passed
        rescue Exception => e
          out[x] = e
        end
      }
      out
    end

    def self.test_simple_struct()
      reader = StringIO::new(
        "\x00\x05" +
        "\x00\x00\x00\x01" +
        "\x00\x84" + "\x00\x00\x00\x04asdf" +
        "\x00\x84" + "\x00\x00\x00\x04asdf")
      hash = Ireul::_unpack_instance(reader)
      raise "Assertion error" if hash[:asdf] != "asdf"
    end

    def self.test_enqueue_type()
      reader = StringIO::new(
        "\x00\x85" + "\x00\x83" +
        "\x00\x00\x00\x00\x00\x00\x00\x09")
      inst = Ireul::Core::ENQUEUE_RESPONSE_TYPE.from_frame(reader)
      raise "Assertion error" if inst.unwrap().value != 9

      reader = StringIO::new(
        "\x00\x86" + "\x00\x82" +
        "\x00\x00\x00\x01")
      inst = Ireul::Core::ENQUEUE_RESPONSE_TYPE.from_frame(reader)
      begin
        inst.unwrap()
        raise "Assertion error"
      rescue EnqueueTrackError::InvalidTrack
        # good!
      end
    end
  end
end
