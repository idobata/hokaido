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

    class Command
      include Celluloid

      def initialize(command, host, port)
        ptyout, ptyin, pid = PTY.getpty(command)
        connection         = Connection.new_link(host, port)

        async.handle_output ptyout, connection
        async.handle_input  ptyin
        async.wait_for_exit pid
      end

      def handle_output(ptyout, connection)
        while chunk = ptyout.readpartial(4096)
          $stdout.write chunk
          connection.async.send chunk
        end
      end

      def handle_input(ptyin)
        while char = $stdin.getch
          ptyin.putc char
        end
      end

      def wait_for_exit(pid)
        Process.waitpid pid

        terminate
      end
    end
  end
end
