File.open ARGV[0] do |f|
    f.each_line do |line|
        num = line[/\d+\.\d+/]
        if num
            puts line.sub(/\d+\.\d+/, (num.to_f * 4).to_s)
        else
            puts line
        end
    end
end