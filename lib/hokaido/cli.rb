require 'celluloid/autostart'
require 'socket'
require 'terminfo'
require 'thor'

module Hokaido
  class CLI < Thor
    class_option :host, aliases: :h, default: '0.0.0.0'
    class_option :port, aliases: :p, default: 4423

    map '-v' => :version

    desc :broadcast, 'Broadcast a session'
    def broadcast(command = ENV['SHELL'])
      Hokaido::Broadcast::Command.run command, *options.values_at(:host, :port)
    end

    desc :server, 'Start server'
    def server
      server = Server.run(*options.values_at(:host, :port))
    rescue Interrupt
      exit
    ensure
      server.terminate if server
    end

    desc :watch, 'Watch a session'
    def watch
      server = TCPSocket.open(*options.values_at(:host, :port))
      server.puts 'watch'

      while chunk = server.readpartial(4096)
        $stdout.write chunk
      end
    rescue Interrupt, EOFError, Errno::EIO
      # do nothing
    ensure
      server.close if server
    end

    desc :version, 'Show hokaido version'
    def version
      say Hokaido::VERSION, :cyan
    end
  end
end
