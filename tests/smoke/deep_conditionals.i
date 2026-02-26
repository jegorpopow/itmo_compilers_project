routine main() is
  var a is 1;
  var b is 1;
  var c is 1;
  var d is 1;
  var e is 1;
  var f is 1;
  var g is 1;
  if a = 1 then
    print 1;
    if b = 1 then
      print 2;
      if c = 1 then
        print 3;
      else
        if d = 1 then
          print 0;
        end;
      end;
    else
      if e = 1 then
        print 0;
      else
        if f = 1 then
          print 0;
        end;
      end;
    end;
    if d = 1 then
      print 4;
      if e = 1 then
        print 5;
        if f = 1 then
          print 6;
          if g = 1 then
            print 7;
          else
            print 0;
          end;
        else
          print 0;
        end;
      else
        print 0;
      end;
    else
      if b = 1 then
        print 0;
      else
        if c = 1 then
          print 0;
        end;
      end;
    end;
  else
    if e = 1 then
      print 0;
    else
      if g = 1 then
        print 0;
      end;
    end;
  end;
end;
