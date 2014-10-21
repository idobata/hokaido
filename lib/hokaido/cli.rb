require 'io/console'
require 'pty'
require 'socket'
require 'terminfo'
require 'thor'
require 'thread'

module Hokaido
  class CLI < Thor
    class_option :host, aliases: :h, default: '0.0.0.0'
    class_option :port, aliases: :p, default: 4423

    desc :sh, 'Start live shell'
    def sh(shell = ENV['SHELL'])
      pty_out, pty_in, pid = *PTY.getpty(shell)
      nonbloq              = Queue.new

      trap :SIGWINCH do
        TermInfo.tiocswinsz pty_in, *TermInfo.screen_size
      end

      Thread.start do
        client = TCPSocket.open(*options.values_at(:host, :port))
        client.puts 'write'

        while c = nonbloq.deq
          client.putc c
        end
      end

      Thread.start do
        while c = pty_out.getc
          $stdout.putc c
          nonbloq.enq c
        end
      end

      Thread.start do
        TermInfo.tiocswinsz pty_in, *TermInfo.screen_size

        while c = $stdin.getch
          pty_in.putc c
        end
      end

      Process.waitpid pid
    end

    desc :server, 'Start server'
    def server
      queue  = Queue.new
      server = TCPServer.open(*options.values_at(:host, :port))

      loop do
        Thread.start server.accept do |client|
          case client.gets.chomp
          when 'write'
            client.puts ':)'

            while c = client.getc
              queue.enq c
            end
          when 'read'
            client.puts '=)'

            while c = queue.deq
              client.putc c
            end
          else
            client.puts ':('
          end

          client.close
        end
      end
    rescue Interrupt
      server.close

      exit
    end

    desc :viewer, 'Open viewer'
    def viewer
      client = TCPSocket.open(*options.values_at(:host, :port))
      client.puts 'read'

      while c = client.getc
        $stdout.putc c
      end
    rescue Interrupt
      client.close

      exit
    end
  end
end
