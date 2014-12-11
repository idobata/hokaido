require 'celluloid'
require 'celluloid/notifications'
require 'socket'

module Hokaido
  class ConnectionHandler
    include Celluloid
    include Celluloid::Notifications

    def initialize(connection)
      @connection = connection

      async.run
    end

    def run
      _, port, host = @connection.peeraddr

      puts "#{host}:#{port} connected"

      case @connection.gets.chomp
      when 'broadcast'
        @connection.puts ':)'

        while chunk = @connection.readpartial(4096)
          publish 'broadcast', chunk
        end
      when 'watch'
        @connection.puts '=)'

        Watcher.new(@connection).link Actor.current

        Kernel.sleep
      else
        @connection.puts ':('
      end
    rescue EOFError, Errno::EIO, Errno::ECONNRESET
      # do nothing
    ensure
      puts "#{host}:#{port} disconnected"

      @connection.close
    end
  end

  class Server
    include Celluloid

    finalizer :shutdown

    def initialize(host, port)
      @server = TCPServer.new(host, port)

      async.run
    end

    def shutdown
      @server.close if @server
    end

    def run
      loop do
        ConnectionHandler.new(@server.accept).link Actor.current
      end
    end
  end
end
