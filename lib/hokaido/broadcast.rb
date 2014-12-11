require 'celluloid'
require 'io/console'
require 'pty'
require 'socket'
require 'terminfo'

module Hokaido
  class Broadcast
    class Connection
      include Celluloid

      def initialize(host, port)
        @socket = TCPSocket.new(host, port)

        @socket.puts 'broadcast'
      end

      def send(data)
        @socket.write data
      end
    end

    class OutputHandler
      include Celluloid

      def initialize(ptyout, connection)
        @ptyout, @connection = ptyout, connection

        async.run
      end

      def run
        while chunk = @ptyout.readpartial(4096)
          $stdout.write chunk
          @connection.async.send chunk
        end

        terminate
      end
    end

    class InputHandler
      include Celluloid

      def initialize(ptyin)
        @ptyin = ptyin

        async.run
      end

      def run
        while char = $stdin.getch
          @ptyin.putc char
        end

        terminate
      end
    end

    class Command
      include Celluloid

      def initialize(command, host, port)
        ptyout, ptyin, pid = PTY.getpty(command)
        connection         = Connection.new_link(host, port)

        OutputHandler.new_link ptyout, connection
        InputHandler.new_link ptyin

        async.wait_for_exit pid
      end

      def wait_for_exit(pid)
        Process.waitpid pid

        terminate
      end
    end
  end
end
