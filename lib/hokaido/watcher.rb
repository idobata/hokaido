require 'celluloid'
require 'celluloid/notifications'

module Hokaido
  class Watcher
    include Celluloid
    include Celluloid::Notifications

    def initialize(socket)
      @socket = socket

      subscribe 'broadcast', :received
    end

    def received(topic, chunk)
      @socket.write chunk
    end
  end
end
