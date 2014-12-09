require 'celluloid'
require 'celluloid/notifications'
require 'socket'

module Hokaido
  class Server
    include Celluloid
    include Celluloid::Notifications

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
        async.handle_connection @server.accept
      end
    end

    def handle_connection(client)
      _, port, host = client.peeraddr

      puts "#{host}:#{port} connected"

      case client.gets.chomp
      when 'broadcast'
        client.puts ':)'

        loop do
          publish 'broadcast', client.readpartial(4096)
        end
      when 'watch'
        client.puts '=)'

        watcher = Watcher.new_link(client)

        loop do
          client.readpartial(4096) # XXX wait for connection closed
        end
      else
        client.puts ':('
      end
    rescue Errno::ECONNRESET
      # do nothing, connetion reset by peer
    ensure
      puts "#{host}:#{port} disconnected"

      client.close
    end
  end
end
