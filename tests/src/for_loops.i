routine array_length (a : array [] integer) : integer;
routine bubble_sort (a : array [] integer, length : integer);

routine sort_and_print_reversed_array(a : array [] integer) is
  bubble_sort(a, a.length)
  for elem in a ..  reversed loop 
    print elem;
  end;
end;

routine array_length (a : array [] integer) : integer is 
  var result is 0;
  for _ in a.. loop
    result := result + 1;
  end;
  return result;
end;

routine bubble_sort (a : array [] integer, length : integer) is 
  for i in 1 .. n loop
    for j in 2 .. i loop 
      if a[j - 1] > a[j] then
        var t is a[j];
        a[j] := a[j - 1];
        a[j - 1] := t;
      end;
    end; 
  end;
end.

routine countdown (n: integer) is 
  for i in n .. 0 reversed loop 
    print i;
  end;
end;

routine count (n : integer) is
  for i in 0 .. n loop
    print i;
  end;
end;

routine main() is
  var arr : array [5] integer; 
  arr[1] = 3;
  arr[2] = 5;
  arr[3] = 1;
  arr[4] = 2;
  arr[5] = 4;
  sort_and_print_reversed_array(arr, arr.length);
  count(3);
  countdown(5);
end;
