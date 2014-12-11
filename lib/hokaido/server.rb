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

        loop do
          publish 'broadcast', @connection.readpartial(4096)
        end
      when 'watch'
        @connection.puts '=)'

        watcher = Watcher.new_link(@connection)

        loop do
          @connection.readpartial(4096) # XXX wait for connection closed
        end
      else
        @connection.puts ':('
      end
    rescue Errno::ECONNRESET
      # do nothing, connetion reset by peer
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
        ConnectionHandler.new_link @server.accept
      end
    end
  end
end
