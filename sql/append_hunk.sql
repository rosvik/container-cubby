UPDATE hunks SET last_byte = last_byte + ?2, data = data || ?3 WHERE id = ?1;
