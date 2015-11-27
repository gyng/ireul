require 'stringio'

module Ireul
  class Unit
    class << self
      def new()
        return Unit
      end
    end

    def self.from_frame(buffer)
      Unit
    end
  end

  class Result
    class Bound
      def from_frame(buffer)
        result = Result::allocate()
        type_byte = buffer.read(1)
        case type_byte.unpack('C')[0]
        when 0
          ok_fac = @ok_fac
          result.instance_eval {
            @ok = ok_fac.call(buffer)
          }
        when 1
          err_fac = @err_fac
          result.instance_eval {
            @err = err_fac.call(buffer)
          }
        else
          raise KeyError, "ValueError, bad read: #{type_byte.inspect}"
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

    VARIANTS = [
      EnqueueTrack,
      FastForward,
    ]

    SYMBOLS = {
      :enqueue_track => EnqueueTrack,
      :fast_forward => FastForward,
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
      value_buf = buffer.read(8)

      handle = Handle::allocate()
      handle.instance_eval {
        @value = value_buf.unpack('Q>')[0]
      }
      handle
    end

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
      variant_id = buffer.read(4).unpack('N')[0]
      variant_type = EnqueueTrackError::from_id(variant_id)
      msg_len = buffer.read(4).unpack('N')[0]
      variant_type::new(buffer.read(msg_len))
    end
  end

  module FastForwardError
    include Enum

    VARIANTS = []

    def self.from_frame(buffer)
      variant_id = buffer.read(4).unpack('N')[0]
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
    # an opaque (to the server) string specified while enqueuing
    # the primary key used in a song database would make sense here
    attr_reader :id

    # an opaque (to the client) u64 allowing queue set operations
    attr_reader :handle

    # the sample rate in Hz
    attr_reader :sample_rate

    # the number of samples in the currently playing track
    attr_reader :length_samples

    def self.from_hash(hash)
      track = Track::allocate()
      track.instance_eval {
        @id = hash[:id]
        @handle = hash[:handle],
        @sample_rate = hash[:sample_rate]
        @length_samples = hash[:length_samples]
      }
      track
    end
  end

  class CoreStatus
    attr_reader :track

    # the number of samples that have been sent to the server
    attr_reader :track_position

    def self.from_hash(hash)
      status = CoreStatus::allocate()
      status.instance_eval {
        @track = hash[:track]
        @track_position = hash[:track_position]
      }
      status
    end

    def position()
      # in seconds
      self.track_position / self.track.sample_rate
    end
  end

  class Core
    ENQUEUE_RESPONSE_TYPE = Result::create_type(Handle, EnqueueTrackError)
    FAST_FORWARD_RESPONSE_TYPE = Result::create_type(Unit, FastForwardError)

    def initialize(socket)
      @socket = socket
    end

    # Accepts an ogg file in the form of a string
    def enqueue(track) # -> Handle
      send_frame(RequestType::EnqueueTrack, track)

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

      message_buffer = [fast_forward.to_int()].pack('N')
      send_frame(RequestType::FastForward, message_buffer)

      rx_frame = recv_frame()
      frame = StringIO::new(rx_frame)
      result = self.class::FAST_FORWARD_RESPONSE_TYPE.from_frame(frame)

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
end
