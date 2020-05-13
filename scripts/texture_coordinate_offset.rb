File.open ARGV[0] do |f|
  f.each_line do |line|
    # We don't touch the line if it has a decimal
    unless line.include? "."
      num = line[/\d+/]
      if num
        fl = num.to_f
        if line.include? " x:" or line.include? " y:"
          fl += 0.01
        elsif line.include? " width:" or line.include? " height:"
          fl -= 0.02
        else
          fl = fl.to_i
        end
        puts line.sub(/\d+/, fl.to_s)
      else
        puts line
      end
    end
  end
end