routine main() is
  var i is 0;
  while i < 2 loop
    var j is 0;
    while j < 2 loop
      var k is 0;
      while k < 2 loop
        var w is 0;
        while w < 2 loop
          var v is 0;
          while v < 2 loop
            print i;
            print j;
            print k;
            print w;
            print v;
            v := v + 1;
          end;
          w := w + 1;
        end;
        k := k + 1;
      end;
      j := j + 1;
    end;
    i := i + 1;
  end;
end;
